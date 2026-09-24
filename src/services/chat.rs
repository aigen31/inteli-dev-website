//! Чатбот: маршрутизация вопросов (preset / free / analysis / lead_request).
//!
//! Источники ответа (см. chatbot.md):
//! - preset/availability → прямой ответ из контента (без LLM, <50ms)
//! - free/analysis → LLM (DeepSeek) с семантическим контекстом из OpenViking
//! - при недоступности LLM → fallback-ответ с контактами

use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::sqlite::SqlitePool;

use crate::cache::InMemoryCache;
use crate::error::{AppError, AppResult};
use crate::limits::Limits;
use crate::llm::prompt::{
    analysis_prompt, build_context_string, chatbot_system_prompt, fallback_answer, ContextSettings,
};
use crate::llm::ChatProvider;
use crate::memory::{Availability, OpenVikingClient};
use crate::storage::chat::{self, NewChat};

/// Входной запрос чата (POST /api/chat).
#[derive(Debug, Clone, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub question_type: Option<String>, // preset | free | analysis | lead_request | availability
    #[serde(default)]
    pub preset_index: Option<usize>,
    #[serde(default)]
    pub url: Option<String>,
}

/// Максимальная длина URL в запросе анализа (защита от раздувания промпта).
const ANALYSIS_URL_MAX_CHARS: usize = 300;
/// Максимальная длина идентификатора сессии (защита БД от мусора).
const SESSION_ID_MAX_CHARS: usize = 64;

impl ChatRequest {
    /// Проверяет запрос до обращения к LLM.
    ///
    /// Браузер ограничивает ввод через `maxlength`, но API можно вызвать
    /// напрямую — поэтому длина проверяется и на сервере.
    pub fn validate(&self) -> AppResult<()> {
        let limits = Limits::get();
        let max = limits.chat_message_max_chars;

        if self.message.trim().is_empty() {
            return Err(AppError::Validation("сообщение не может быть пустым".into()));
        }
        // Считаем именно символы, а не байты: кириллица в UTF-8 занимает 2 байта.
        let len = self.message.chars().count();
        if len > max {
            return Err(AppError::Validation(format!(
                "сообщение слишком длинное: {len} из {max} символов"
            )));
        }
        if let Some(url) = self.url.as_deref() {
            if url.chars().count() > ANALYSIS_URL_MAX_CHARS {
                return Err(AppError::Validation("ссылка слишком длинная".into()));
            }
        }
        if let Some(sid) = self.session_id.as_deref() {
            if sid.chars().count() > SESSION_ID_MAX_CHARS {
                return Err(AppError::Validation("некорректный идентификатор сессии".into()));
            }
        }
        Ok(())
    }
}

/// Ответ чата (POST /api/chat).
#[derive(Debug, Clone, Serialize)]
pub struct ChatResponse {
    pub answer: String,
    pub source: String, // openviking_direct | llm | llm_fallback | cached
    pub suggested_next: Vec<SuggestedAction>,
}

/// Кнопка-подсказка для следующего шага.
#[derive(Debug, Clone, Serialize)]
pub struct SuggestedAction {
    pub text: String,
    #[serde(rename = "type")]
    pub kind: String,
}

/// Тип вопроса после нормализации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuestionKind {
    Who,
    Services,
    Pricing,
    LocalInference,
    Analysis,
    Availability,
    LeadRequest,
    Free,
}

impl QuestionKind {
    /// Тип прямого ответа для preset-кнопки.
    ///
    /// Неизвестный индекс уходит в LLM (`Free`), а не отвечает наугад:
    /// лучше потратить токены, чем отдать ответ не на тот вопрос.
    fn from_preset(index: usize) -> Self {
        match index {
            0 => Self::Who,
            1 => Self::Services,
            2 => Self::Pricing,
            3 => Self::LocalInference,
            _ => Self::Free,
        }
    }
}

/// Кнопка-вопрос: подпись и способ обработки.
///
/// Единый источник правды для страницы `/chat`, hero-терминала на главной и
/// роутера. Раньше подписи и индексы жили в трёх местах и разошлись: кнопка
/// «Сколько стоит?» на главной отправляла `preset_index = 2`, который на
/// странице `/chat` означал «Локальный инференс — что это?». Клиент спрашивал
/// цену и получал лекцию про инференс, а функция с ценами вообще осталась
/// мёртвым кодом.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preset {
    /// `preset_index` в запросе к `/api/chat`.
    pub index: usize,
    /// Текст на кнопке. Он же уходит в `message`.
    pub label: &'static str,
    /// `question_type` в запросе: preset | analysis | availability | lead_request.
    pub kind: &'static str,
}

/// Все кнопки страницы `/chat`, в порядке показа.
pub const PRESETS: &[Preset] = &[
    Preset { index: 0, label: "Кто вы?", kind: "preset" },
    Preset { index: 1, label: "Чем занимаетесь?", kind: "preset" },
    Preset { index: 2, label: "Сколько стоит?", kind: "preset" },
    Preset { index: 3, label: "Локальный инференс — что это?", kind: "preset" },
    Preset { index: 4, label: "Проанализируйте мой сайт", kind: "analysis" },
    Preset { index: 5, label: "Когда свободны?", kind: "availability" },
    Preset { index: 6, label: "Оставить заявку", kind: "lead_request" },
];

/// Подмножество кнопок для hero-терминала на главной: там место под три.
pub const HERO_PRESET_INDICES: &[usize] = &[0, 1, 2];

/// Кнопки hero-терминала в порядке [`HERO_PRESET_INDICES`].
pub fn hero_presets() -> impl Iterator<Item = &'static Preset> {
    HERO_PRESET_INDICES
        .iter()
        .filter_map(|index| PRESETS.iter().find(|p| p.index == *index))
}

/// Сервис чата.
pub struct ChatService {
    llm: Arc<dyn ChatProvider>,
    memory: Arc<OpenVikingClient>,
    cache: Arc<InMemoryCache>,
    db: SqlitePool,
    /// Бюджет RAG-контекста (сколько тратим входных токенов на справку).
    context: ContextSettings,
    /// Сколько результатов запрашивать у OpenViking (обычно == `context.top_k`).
    search_limit: usize,
}

impl ChatService {
    pub fn new(
        llm: Arc<dyn ChatProvider>,
        memory: Arc<OpenVikingClient>,
        cache: Arc<InMemoryCache>,
        db: SqlitePool,
    ) -> Self {
        Self::with_context(llm, memory, cache, db, ContextSettings::default())
    }

    /// Конструктор с настраиваемым бюджетом контекста (значения из config.toml).
    pub fn with_context(
        llm: Arc<dyn ChatProvider>,
        memory: Arc<OpenVikingClient>,
        cache: Arc<InMemoryCache>,
        db: SqlitePool,
        context: ContextSettings,
    ) -> Self {
        Self {
            llm,
            memory,
            cache,
            db,
            context,
            // Просим у поиска чуть больше, чем подмешаем: часть отсеется
            // по порогу релевантности.
            search_limit: context.top_k.saturating_mul(2).max(1),
        }
    }

    /// Ответ уже в кэше — запрос не потратит токены LLM.
    ///
    /// Используется, чтобы не списывать часовую квоту за повторный вопрос.
    pub fn is_cached(&self, req: &ChatRequest) -> bool {
        matches!(resolve_kind(req), QuestionKind::Free)
            && self.cache.get(&cache_key(&req.message)).is_some()
    }

    /// Обрабатывает запрос и возвращает ответ.
    pub async fn answer(
        &self,
        req: ChatRequest,
        client_hash: Option<String>,
    ) -> AppResult<ChatResponse> {
        req.validate()?;

        let started = Instant::now();
        let kind = resolve_kind(&req);

        let (answer, source) = match kind {
            QuestionKind::Who => (answer_who(), "openviking_direct".to_string()),
            QuestionKind::Services => (answer_services(), "openviking_direct".to_string()),
            QuestionKind::Pricing => (answer_pricing(), "openviking_direct".to_string()),
            QuestionKind::LocalInference => (answer_local_inference(), "openviking_direct".to_string()),
            QuestionKind::Availability => (answer_availability(), "openviking_direct".to_string()),
            QuestionKind::LeadRequest => (answer_lead_request(), "openviking_direct".to_string()),
            QuestionKind::Analysis => self.answer_analysis(&req).await?,
            QuestionKind::Free => self.answer_free(&req).await?,
        };

        let response = ChatResponse {
            answer,
            source,
            suggested_next: suggested_next(kind),
        };

        let response_time_ms = Some(started.elapsed().as_millis() as i64);
        self.save_chat(&req, &response, client_hash, response_time_ms)
            .await?;

        Ok(response)
    }

    /// Анализ сайта: требует URL, иначе просит его прислать.
    async fn answer_analysis(&self, req: &ChatRequest) -> AppResult<(String, String)> {
        let url = req.url.as_deref().filter(|u| !u.trim().is_empty());
        let Some(url) = url else {
            return Ok((
                "Чтобы я проанализировал ваш сайт, пришлите ссылку на него (например, example.com)."
                    .to_string(),
                "openviking_direct".to_string(),
            ));
        };

        if !crate::utils::validator::looks_like_url(url) {
            return Ok((
                "Похоже, ссылка некорректна. Пришлите адрес вида example.com или https://example.com."
                    .to_string(),
                "openviking_direct".to_string(),
            ));
        }

        let content = crate::memory::content::SiteContent::get();
        match self
            .llm
            .chat_with_system(analysis_prompt(url), url.to_string())
            .await
        {
            Ok(answer) => Ok((answer, "llm".to_string())),
            Err(e) => {
                tracing::warn!("analysis LLM failed: {e}");
                Ok((
                    fallback_answer(&content.profile),
                    "llm_fallback".to_string(),
                ))
            }
        }
    }

    /// Свободный вопрос: кэш → семантический поиск → LLM → кэш.
    async fn answer_free(&self, req: &ChatRequest) -> AppResult<(String, String)> {
        let cache_key = cache_key(&req.message);
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok((cached, "cached".to_string()));
        }

        let content = crate::memory::content::SiteContent::get();

        // Семантический контекст из OpenViking (при ошибке — пустой контекст).
        let context = match self.memory.semantic_search(&req.message, self.search_limit).await {
            Ok(results) => build_context_string(&results, &self.context),
            Err(e) => {
                tracing::debug!("OpenViking search failed, empty context: {e}");
                String::new()
            }
        };

        let system = chatbot_system_prompt(&content.profile);
        let user = if context.is_empty() {
            req.message.clone()
        } else {
            format!(
                "Контекст из базы знаний:\n{context}\n\nВопрос посетителя:\n{}",
                req.message
            )
        };

        let (answer, source) = match self.llm.chat_with_system(system, user).await {
            Ok(answer) => (answer, "llm".to_string()),
            Err(e) => {
                tracing::warn!("free-form LLM failed: {e}");
                (
                    fallback_answer(&content.profile),
                    "llm_fallback".to_string(),
                )
            }
        };

        if source == "llm" {
            self.cache.set(cache_key, answer.clone());
        }

        Ok((answer, source))
    }

    /// Сохраняет диалог в SQLite для аналитики (см. admin-panel.md).
    async fn save_chat(
        &self,
        req: &ChatRequest,
        response: &ChatResponse,
        client_hash: Option<String>,
        response_time_ms: Option<i64>,
    ) -> AppResult<()> {
        if let Err(e) = chat::insert(
            &self.db,
            NewChat {
                user_message: req.message.clone(),
                bot_response: response.answer.clone(),
                question_type: Some(normalized_type(req)),
                session_id: req.session_id.clone(),
                client_hash,
                response_time_ms,
            },
        )
        .await
        {
            // Сбой записи истории не должен ломать ответ пользователю.
            tracing::warn!("failed to persist chat: {e}");
        }
        Ok(())
    }
}

fn normalized_type(req: &ChatRequest) -> String {
    req.question_type
        .clone()
        .unwrap_or_else(|| "free".to_string())
}

/// Тратит ли запрос токены LLM.
///
/// Используется API-слоем, чтобы списывать часовую квоту только за платные
/// ответы: preset/availability/lead_request отдаются прямо из контента.
pub fn requires_llm(req: &ChatRequest) -> bool {
    matches!(
        resolve_kind(req),
        QuestionKind::Analysis | QuestionKind::Free
    )
}

/// Ключ кэша ответов LLM.
///
/// Сообщение нормализуется (регистр, лишние пробелы), иначе «Сколько стоит?» и
/// «сколько  стоит?» уходят в модель дважды и дважды оплачиваются.
fn cache_key(message: &str) -> String {
    let normalized = message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    format!("chat:{}", sha256_hex(&normalized))
}

fn resolve_kind(req: &ChatRequest) -> QuestionKind {
    match req.question_type.as_deref() {
        Some("analysis") => QuestionKind::Analysis,
        Some("availability") => QuestionKind::Availability,
        Some("lead_request") => QuestionKind::LeadRequest,
        Some("preset") => QuestionKind::from_preset(req.preset_index.unwrap_or(0)),
        _ => QuestionKind::Free,
    }
}

fn suggested_next(kind: QuestionKind) -> Vec<SuggestedAction> {
    match kind {
        QuestionKind::LeadRequest => vec![SuggestedAction {
            text: "Перейти к форме заявки".into(),
            kind: "link:/contact".into(),
        }],
        _ => vec![
            SuggestedAction {
                text: "Рассчитать стоимость проекта".into(),
                kind: "lead_request".into(),
            },
            SuggestedAction {
                text: "Посмотреть примеры работ".into(),
                kind: "link:/projects".into(),
            },
        ],
    }
}

/// «Кто вы?» — кто это и чем полезен, без пересказа всего профиля.
///
/// Раньше здесь был полный список компетенций и приглашение оставить заявку:
/// ответ на «кто вы?» превращался в простыню, а сам вопрос оставался без
/// прямого ответа. Компетенции целиком видны на главной, а кнопки следующего
/// шага рисует UI (`suggested_next`).
fn answer_who() -> String {
    let p = &crate::memory::content::SiteContent::get().profile;
    // Три ключевые компетенции: этого достаточно, чтобы понять профиль.
    let focus = p
        .skills
        .iter()
        .take(3)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "{name} — {title}. {years} лет опыта, {projects}+ проектов.\n\n\
         {focus}.\n\
         {location}.",
        name = p.name,
        title = p.title,
        years = p.experience_years,
        projects = p.projects_completed,
        location = p.location,
    )
}

/// «Чем занимаетесь?» — список услуг по одной строке.
///
/// Полные описания услуг — это шесть абзацев на два экрана. В чате нужен
/// список, по которому за секунду видно, подходит ли специалист под задачу;
/// подробности клиент читает на `/services`.
fn answer_services() -> String {
    let c = crate::memory::content::SiteContent::get();
    let mut out = String::from("Чем занимаюсь:");
    for s in &c.services {
        // Разделитель — двоеточие, а не тире: описания сами содержат тире
        // («ComfyUI Mass Production — Массовая генерация…»), и второе тире
        // подряд читалось бы как опечатка.
        out.push_str(&format!(
            "\n• {}: {}",
            s.title,
            first_sentence(&s.description)
        ));
    }
    out
}

/// «Сколько стоит?» — вилка цен по услугам.
fn answer_pricing() -> String {
    let c = crate::memory::content::SiteContent::get();
    let mut out = String::from("Цены — ориентировочно, зависят от объёма:");
    for s in &c.services {
        out.push_str(&format!("\n• {}: {}", s.title, price_range(s)));
    }
    out.push_str("\n\nТочную стоимость считаю после описания задачи.");
    out
}

/// Цена услуги одной строкой.
fn price_range(service: &crate::memory::content::Service) -> String {
    match (&service.price_from, &service.price_to) {
        (Some(from), Some(to)) => format!("{from} – {to}"),
        (Some(from), None) => format!("от {from}"),
        (None, Some(to)) => format!("до {to}"),
        (None, None) => "по запросу".to_string(),
    }
}

/// Первое предложение текста — короткая суть без «воды».
///
/// Описания услуг написаны по схеме «короткая суть. Подробности.» — первое
/// предложение и есть то, что нужно в списке.
fn first_sentence(text: &str) -> String {
    let trimmed = text.trim();
    match trimmed.find(". ") {
        Some(idx) if idx > 0 => trimmed[..=idx].trim_end().to_string(),
        _ => trimmed.to_string(),
    }
}

/// «Локальный инференс — что это?» — определение и что это даёт на практике.
fn answer_local_inference() -> String {
    "Локальный инференс — это когда AI-модель работает на вашем железе, а не в облаке.\n\n\
     Что это даёт:\n\
     • Данные не покидают ваш сервер\n\
     • Нет абонплаты за API и лимитов провайдера\n\
     • Нет сетевых задержек — ответ мгновенный\n\n\
     Мой стек: GPU-кластер (RTX 5080 + RTX 3060, 64 GB RAM), модели Qwen 3.6/3.8 до 35B."
        .to_string()
}

fn answer_availability() -> String {
    // Живой статус, а не контент: владелец меняет его из Telegram-бота,
    // и AI-ассистент должен отвечать то же, что показывает бейдж на сайте.
    format_availability(&crate::settings::SiteSettings::availability())
}

/// Форматирует статус занятости в текст ответа чата.
///
/// Подлежащее и факты берутся из словаря статуса (`services::status`) — того
/// же, что кормит бейдж в шапке, `/api/status` и Telegram-бота. Поэтому ответ
/// ассистента не может разойтись с надписью на сайте.
pub(crate) fn format_availability(a: &Availability) -> String {
    use crate::services::status;

    // Один факт на строку: «Ближайший слот — немедленно — есть свободные слоты»
    // с двойным тире читалось как ошибка вёрстки.
    let mut lines = vec![format!("{}.", status::status_line(&a.status))];
    lines.extend(status::facts(a).into_iter().map(|fact| format!("{fact}.")));
    lines.join("\n")
}

fn answer_lead_request() -> String {
    "Опишите задачу в форме заявки: имя, контакт и что нужно сделать. Отвечу в течение 24 часов."
        .to_string()
}

fn sha256_hex(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    hex::encode(digest)[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use crate::llm::Message;
    use crate::memory::content::{fallback_content, SiteContent};
    use crate::storage::init_memory_pool;

    struct MockProvider {
        fail: bool,
    }

    #[async_trait::async_trait]
    impl ChatProvider for MockProvider {
        async fn chat_with_history(&self, _messages: Vec<Message>) -> AppResult<String> {
            if self.fail {
                Err(AppError::EmptyLlmResponse)
            } else {
                Ok("MOCK ANSWER".to_string())
            }
        }
    }

    async fn service(provider_fails: bool) -> ChatService {
        let pool = init_memory_pool().await.unwrap();
        let llm: Arc<dyn ChatProvider> = Arc::new(MockProvider {
            fail: provider_fails,
        });
        let memory = Arc::new(OpenVikingClient::new(
            "http://127.0.0.1:1",
            std::time::Duration::from_millis(10),
        ));
        let cache = Arc::new(InMemoryCache::new(std::time::Duration::from_secs(60)));
        ChatService::new(llm, memory, cache, pool)
    }

    fn ensure_content() {
        // Повторный set — no-op (OnceLock), поэтому безопасно вызывать в каждом тесте.
        SiteContent::set_global(fallback_content());
    }

    fn preset_req(index: usize) -> ChatRequest {
        ChatRequest {
            message: "preset".into(),
            session_id: None,
            question_type: Some("preset".into()),
            preset_index: Some(index),
            url: None,
        }
    }

    fn free_req() -> ChatRequest {
        ChatRequest {
            message: "как увеличить трафик?".into(),
            session_id: None,
            question_type: None,
            preset_index: None,
            url: None,
        }
    }

    #[tokio::test]
    async fn preset_who_does_not_need_llm() {
        ensure_content();
        let svc = service(true).await; // LLM сломан — preset всё равно работает
        let resp = svc.answer(preset_req(0), None).await.unwrap();
        assert_eq!(resp.source, "openviking_direct");
        assert!(resp.answer.contains("Евгений Биль"));
    }

    #[tokio::test]
    async fn preset_local_inference_returns_explanation() {
        ensure_content();
        let svc = service(true).await;
        let resp = svc.answer(preset_req(3), None).await.unwrap();
        assert_eq!(resp.source, "openviking_direct");
        assert!(resp.answer.contains("инференс") || resp.answer.contains("Инференс"));
    }

    /// Регрессия: кнопка «Сколько стоит?» отдавала лекцию про локальный
    /// инференс, потому что индекс 2 в hero-терминале на главной значил не то
    /// же, что индекс 2 на странице /chat.
    #[tokio::test]
    async fn pricing_preset_returns_prices() {
        ensure_content();
        let svc = service(true).await;

        let index = |label: &str| {
            PRESETS
                .iter()
                .find(|p| p.label == label)
                .expect("кнопка есть в списке")
                .index
        };

        let resp = svc.answer(preset_req(index("Сколько стоит?")), None).await.unwrap();
        assert_eq!(resp.source, "openviking_direct");
        // Цена каждой услуги, а не объяснение технологии.
        for service in &crate::memory::content::SiteContent::get().services {
            assert!(
                resp.answer.contains(&service.title),
                "в ответе о ценах нет услуги «{}»: {}",
                service.title,
                resp.answer
            );
        }
        assert!(resp.answer.contains("₽"), "нет ни одной цены: {}", resp.answer);
        assert!(
            !resp.answer.contains("инференс"),
            "ответ о ценах ушёл в тему инференса: {}",
            resp.answer
        );
    }

    /// Кнопки hero-терминала должны существовать в общем списке: иначе на
    /// главной снова появится кнопка, которой сервер не знает.
    #[test]
    fn hero_presets_are_a_subset_of_all_presets() {
        let hero: Vec<usize> = hero_presets().map(|p| p.index).collect();
        assert_eq!(hero, HERO_PRESET_INDICES.to_vec());
        assert!(hero.len() >= 3);
    }

    /// Каждая кнопка из общего списка обязана иметь прямой ответ (или явно
    /// уходить в LLM). Тест ловит добавление кнопки без обработки в роутере.
    #[test]
    fn every_preset_button_routes_to_a_known_kind() {
        for preset in PRESETS {
            let req = ChatRequest {
                message: preset.label.into(),
                session_id: None,
                question_type: Some(preset.kind.into()),
                preset_index: Some(preset.index),
                url: None,
            };
            let kind = resolve_kind(&req);

            match preset.kind {
                // Прямые ответы: не Free, иначе кнопка молча уйдёт в модель.
                "preset" => assert_ne!(
                    kind,
                    QuestionKind::Free,
                    "preset {} «{}» не имеет прямого ответа",
                    preset.index,
                    preset.label
                ),
                "analysis" => assert_eq!(kind, QuestionKind::Analysis),
                "availability" => assert_eq!(kind, QuestionKind::Availability),
                "lead_request" => assert_eq!(kind, QuestionKind::LeadRequest),
                other => panic!("неизвестный question_type «{other}» у кнопки «{}»", preset.label),
            }
        }
    }

    /// Типовые ответы должны быть короткими: клиент читает их с телефона.
    #[tokio::test]
    async fn direct_answers_stay_short_and_have_no_cta_tail() {
        ensure_content();
        let svc = service(true).await;

        for preset in PRESETS.iter().filter(|p| p.kind == "preset") {
            let resp = svc.answer(preset_req(preset.index), None).await.unwrap();

            // 900 символов — это примерно экран телефона. Всё, что длиннее,
            // возвращает нас к простыням, из-за которых ответы и переписывали.
            assert!(
                resp.answer.chars().count() < 900,
                "ответ на «{}» слишком длинный ({} символов)",
                preset.label,
                resp.answer.chars().count()
            );

            for fluff in ["Оставьте заявку", "оставьте заявку", "Хотите обсудить", "Отлично!"] {
                assert!(
                    !resp.answer.contains(fluff),
                    "в ответе на «{}» осталась вода «{fluff}»: {}",
                    preset.label,
                    resp.answer
                );
            }
        }
    }

    /// «Чем занимаетесь?» перечисляет услуги, но не вываливает их описания.
    #[tokio::test]
    async fn services_answer_lists_titles_without_full_descriptions() {
        ensure_content();
        let svc = service(true).await;

        let index = PRESETS.iter().find(|p| p.label == "Чем занимаетесь?").unwrap().index;
        let resp = svc.answer(preset_req(index), None).await.unwrap();

        let services = crate::memory::content::SiteContent::get().services.clone();
        assert_eq!(services.len(), 6);
        for service in &services {
            assert!(resp.answer.contains(&service.title), "нет услуги «{}»", service.title);
            // Полное описание в ответе не помещается — и не должно.
            assert!(
                !resp.answer.contains(&service.description),
                "описание «{}» попало в ответ целиком",
                service.title
            );
        }
    }

    #[test]
    fn first_sentence_cuts_at_the_first_period() {
        assert_eq!(
            first_sentence("Короткая суть. Дальше подробности."),
            "Короткая суть."
        );
        // Точка без пробела (версия модели, домен) предложение не разрывает.
        assert_eq!(
            first_sentence("Работает на Qwen 3.6 и далее."),
            "Работает на Qwen 3.6 и далее."
        );
        assert_eq!(first_sentence("Без точки"), "Без точки");
        assert_eq!(first_sentence("  с пробелами.  "), "с пробелами.");
    }

    #[tokio::test]
    async fn free_text_uses_llm_when_available() {
        ensure_content();
        let svc = service(false).await;
        let resp = svc.answer(free_req(), None).await.unwrap();
        assert_eq!(resp.answer, "MOCK ANSWER");
        assert_eq!(resp.source, "llm");
    }

    #[tokio::test]
    async fn free_text_falls_back_when_llm_down() {
        ensure_content();
        let svc = service(true).await;
        let resp = svc.answer(free_req(), None).await.unwrap();
        assert_eq!(resp.source, "llm_fallback");
        assert!(resp.answer.contains("Telegram"));
    }

    #[tokio::test]
    async fn analysis_without_url_asks_for_url() {
        ensure_content();
        let svc = service(false).await;
        let req = ChatRequest {
            message: "анализ".into(),
            session_id: None,
            question_type: Some("analysis".into()),
            preset_index: None,
            url: None,
        };
        let resp = svc.answer(req, None).await.unwrap();
        assert!(resp.answer.contains("ссылку"));
    }

    #[tokio::test]
    async fn lead_request_returns_cta() {
        ensure_content();
        let svc = service(true).await;
        let req = ChatRequest {
            message: "заявка".into(),
            session_id: None,
            question_type: Some("lead_request".into()),
            preset_index: None,
            url: None,
        };
        let resp = svc.answer(req, None).await.unwrap();
        // Ответ должен объяснять следующий шаг и срок, без вводных «Отлично!».
        assert!(resp.answer.contains("заявк"), "получено: {}", resp.answer);
        assert!(resp.answer.contains("24"), "получено: {}", resp.answer);
        assert!(!resp.answer.starts_with("Отлично"));
        assert!(!resp.suggested_next.is_empty());
    }

    #[tokio::test]
    async fn chat_is_persisted_to_db() {
        ensure_content();
        let svc = service(true).await;
        svc.answer(preset_req(0), None).await.unwrap();
        let rows = crate::storage::chat::recent(&svc.db, 10).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].question_type.as_deref(), Some("preset"));
    }

    #[tokio::test]
    async fn empty_message_is_rejected() {
        ensure_content();
        let svc = service(true).await;
        let mut req = free_req();
        req.message = "   ".into();
        let err = svc.answer(req, None).await.unwrap_err();
        assert!(matches!(err, AppError::Validation(_)), "получено: {err}");
    }

    #[tokio::test]
    async fn overly_long_message_is_rejected() {
        ensure_content();
        let svc = service(true).await;
        let mut req = free_req();
        let max = Limits::get().chat_message_max_chars;
        req.message = "а".repeat(max + 1);
        let err = svc.answer(req, None).await.unwrap_err();
        assert!(matches!(err, AppError::Validation(_)), "получено: {err}");
    }

    #[tokio::test]
    async fn message_at_the_limit_is_accepted() {
        ensure_content();
        let svc = service(true).await;
        let mut req = free_req();
        let max = Limits::get().chat_message_max_chars;
        req.message = "а".repeat(max);
        assert!(svc.answer(req, None).await.is_ok());
    }

    #[tokio::test]
    async fn repeated_question_hits_cache_and_saves_tokens() {
        ensure_content();
        let svc = service(false).await;

        let first = svc.answer(free_req(), None).await.unwrap();
        assert_eq!(first.source, "llm");

        // Тот же вопрос другим регистром и с лишними пробелами.
        let mut req = free_req();
        req.message = "Как   увеличить ТРАФИК?".into();
        assert!(svc.is_cached(&req), "нормализованный ключ должен совпасть");

        let second = svc.answer(req, None).await.unwrap();
        assert_eq!(second.source, "cached");
        assert_eq!(second.answer, first.answer);
    }

    #[test]
    fn requires_llm_marks_only_token_spending_requests() {
        // free — в модель.
        assert!(requires_llm(&free_req()));

        // analysis — в модель.
        let mut analysis = free_req();
        analysis.question_type = Some("analysis".into());
        assert!(requires_llm(&analysis));

        // Кнопки без ИИ не должны тратить токены, а кнопка анализа — должна.
        for preset in PRESETS {
            let req = ChatRequest {
                message: preset.label.into(),
                session_id: None,
                question_type: Some(preset.kind.into()),
                preset_index: Some(preset.index),
                url: Some("example.com".into()),
            };
            let spends = matches!(preset.kind, "analysis");
            assert_eq!(
                requires_llm(&req),
                spends,
                "кнопка «{}» (kind={}) неверно классифицирована",
                preset.label,
                preset.kind
            );
        }

        // lead_request / availability — прямые.
        let mut lead = free_req();
        lead.question_type = Some("lead_request".into());
        assert!(!requires_llm(&lead));
    }

    #[test]
    fn cache_key_ignores_case_and_extra_spaces() {
        assert_eq!(cache_key("Сколько  стоит?"), cache_key("сколько стоит? "));
        assert_ne!(cache_key("сколько стоит?"), cache_key("что вы умеете?"));
    }
}

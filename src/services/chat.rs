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
use crate::error::AppResult;
use crate::llm::prompt::{
    analysis_prompt, build_context_string, chatbot_system_prompt, fallback_answer,
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
    Analysis,
    Availability,
    LeadRequest,
    Free,
}

/// Сервис чата.
pub struct ChatService {
    llm: Arc<dyn ChatProvider>,
    memory: Arc<OpenVikingClient>,
    cache: Arc<InMemoryCache>,
    db: SqlitePool,
}

impl ChatService {
    pub fn new(
        llm: Arc<dyn ChatProvider>,
        memory: Arc<OpenVikingClient>,
        cache: Arc<InMemoryCache>,
        db: SqlitePool,
    ) -> Self {
        Self {
            llm,
            memory,
            cache,
            db,
        }
    }

    /// Обрабатывает запрос и возвращает ответ.
    pub async fn answer(
        &self,
        req: ChatRequest,
        client_hash: Option<String>,
    ) -> AppResult<ChatResponse> {
        let started = Instant::now();
        let kind = resolve_kind(&req);

        let (answer, source) = match kind {
            QuestionKind::Who => (answer_who(), "openviking_direct".to_string()),
            QuestionKind::Services => (answer_services(), "openviking_direct".to_string()),
            QuestionKind::Pricing => (answer_pricing(), "openviking_direct".to_string()),
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
        let cache_key = format!("chat:{}", sha256_hex(&req.message));
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok((cached, "cached".to_string()));
        }

        let content = crate::memory::content::SiteContent::get();

        // Семантический контекст из OpenViking (при ошибке — пустой контекст).
        let context = match self.memory.semantic_search(&req.message, 5).await {
            Ok(results) => build_context_string(&results),
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

fn resolve_kind(req: &ChatRequest) -> QuestionKind {
    match req.question_type.as_deref() {
        Some("analysis") => QuestionKind::Analysis,
        Some("availability") => QuestionKind::Availability,
        Some("lead_request") => QuestionKind::LeadRequest,
        Some("preset") => match req.preset_index.unwrap_or(0) {
            0 => QuestionKind::Who,
            1 => QuestionKind::Services,
            2 => QuestionKind::Pricing,
            3 => QuestionKind::Analysis,
            4 => QuestionKind::Availability,
            5 => QuestionKind::LeadRequest,
            _ => QuestionKind::Free,
        },
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
                text: "Рассчитать стоимость моего сайта".into(),
                kind: "lead_request".into(),
            },
            SuggestedAction {
                text: "Посмотреть примеры работ".into(),
                kind: "link:/projects".into(),
            },
        ],
    }
}

fn answer_who() -> String {
    let c = crate::memory::content::SiteContent::get();
    let p = &c.profile;
    format!(
        "Я — {name}, {title} с {years}-летним опытом. За это время реализовал {projects}+ проектов.\n\n\
         Ключевые компетенции:\n• {skills}\n\n\
         Хотите обсудить ваш проект? Оставьте заявку — отвечу в течение 24 часов.",
        name = p.name,
        title = p.title,
        years = p.experience_years,
        projects = p.projects_completed,
        skills = p.skills.join("\n• "),
    )
}

fn answer_services() -> String {
    let c = crate::memory::content::SiteContent::get();
    let mut out = String::from("Вот чем я могу помочь:\n\n");
    for s in &c.services {
        out.push_str(&format!("• {} {} — {}\n", s.icon, s.title, s.description));
    }
    out.push_str("\nХотите рассчитать стоимость для вашего проекта? Оставьте заявку.");
    out
}

fn answer_pricing() -> String {
    let c = crate::memory::content::SiteContent::get();
    let mut out = String::from("Ориентировочные расценки:\n\n");
    for s in &c.services {
        let price = match (&s.price_from, &s.price_to) {
            (Some(from), Some(to)) => format!("{from} – {to}"),
            (Some(from), None) => format!("от {from}"),
            (None, Some(to)) => format!("до {to}"),
            (None, None) => "по запросу".to_string(),
        };
        out.push_str(&format!("• {} — {}\n", s.title, price));
    }
    out.push_str("\nТочная стоимость зависит от проекта. Отправьте ссылку на сайт — посчитаю после бесплатного аудита.");
    out
}

fn answer_availability() -> String {
    let c = crate::memory::content::SiteContent::get();
    format_availability(&c.availability)
}

/// Форматирует статус занятости в текст (переиспользуется в status-сервисе).
pub(crate) fn format_availability(a: &Availability) -> String {
    format!(
        "{emoji} Сейчас: {label}.\nВ работе {projects} проектов. Ближайший слот — {slot}.\n\n\
         Если сроки горят — напишите в Telegram, постараемся найти решение.",
        emoji = status_emoji(&a.status),
        label = label_for_status(&a.status),
        projects = a.current_projects,
        slot = a.next_free_slot,
    )
}

/// Эмодзи-индикатор статуса.
pub(crate) fn status_emoji(status: &str) -> &'static str {
    match status {
        "available" => "🟢",
        "busy" => "🟡",
        _ => "🔴",
    }
}

/// Человекочитаемая подпись статуса.
pub(crate) fn label_for_status(status: &str) -> &'static str {
    match status {
        "available" => "свободен для новых проектов",
        "busy" => "ограниченная доступность",
        _ => "полная загрузка",
    }
}

fn answer_lead_request() -> String {
    "Отлично! Оставьте заявку через форму — опишите проект и оставьте контакт. Отвечу в течение 24 часов.\n\n\
     Также можно написать напрямую в Telegram."
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
        assert!(resp.answer.contains("Иван Петров"));
    }

    #[tokio::test]
    async fn preset_pricing_lists_prices() {
        ensure_content();
        let svc = service(true).await;
        let resp = svc.answer(preset_req(2), None).await.unwrap();
        assert!(resp.answer.contains("15 000"));
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
        assert!(resp.answer.contains("заявку"));
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
}

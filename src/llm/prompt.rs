//! Промпты: системный промпт чата, промпт анализа сайта, fallback-ответы.
//!
//! В проде промпты загружаются из OpenViking (`viking://.../prompts/*`). Здесь —
//! встроенные шаблоны, используемые при недоступности OpenViking.

use crate::memory::{AuthorProfile, MemoryResult};

/// Собирает системный промпт чатбота из профиля автора.
///
/// Промпт уходит в модель на КАЖДОМ free-запросе, поэтому он сжат до минимума:
/// все факты сохранены, но убраны повторы (имя/опыт дублировались дважды) и
/// многословие. Это ~⅓ экономии входных токенов без потери качества ответов.
pub fn chatbot_system_prompt(profile: &AuthorProfile) -> String {
    format!(
        r#"Ты — ассистент на сайте {name}. Твоя задача — ответить на вопрос посетителя,
а не рассказать о сайте.

Правила:
1. Первое предложение — прямой ответ по существу вопроса. Без вступлений,
   без «я ассистент», без описания того, чем занимается автор.
2. Не пересказывай услуги, компетенции и цифры, если вопрос не об этом.
   Опыт ({years} лет, {projects}+ проектов) называй только когда спрашивают
   об опыте, надёжности или сроках.
3. Не повторяй то, что уже прозвучало в этом диалоге.
4. Длина — 1–3 предложения. Список уместен, только если просят перечислить
   или сравнить.
5. Цены бери только из контекста ниже. Если цен нет — скажи, что стоимость
   считается после описания задачи, и всё.
6. Не знаешь ответа — скажи прямо, без общих слов.
7. Заявку предлагай только когда вопрос про заказ, цену, сроки или когда
   ответа нет. Не заканчивай каждый ответ призывом.
8. По-русски, без канцелярита и без «отлично, вопрос!».

Что реально умеет автор (справка, не для пересказа):
• Приватные AI-системы: локальный инференс на своём железе, данные не уходят
  наружу, без абонплаты за API.
• MCP-серверы и AI Skills: интеграция ИИ в чужой код, голосовые ассистенты.
• Fullstack: Symfony, Laravel, React, Vue, REST, Docker, CI/CD.
• ComfyUI: автоматизированные пайплайны генерации изображений."#,
        name = profile.name,
        years = profile.experience_years,
        projects = profile.projects_completed,
    )
}

/// Бюджет RAG-контекста: сколько результатов и символов уходит в промпт.
///
/// Контекст — вторая по величине статья входных токенов после системного
/// промпта, поэтому он ограничен и по числу фрагментов, и по суммарной длине.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextSettings {
    /// Сколько фрагментов максимум подмешиваем.
    pub top_k: usize,
    /// Сколько символов берём из каждого фрагмента.
    pub snippet_chars: usize,
    /// Жёсткий потолок всего контекста в символах.
    pub max_chars: usize,
    /// Порог релевантности: что ниже — выбрасываем как шум.
    pub min_score: f64,
}

impl Default for ContextSettings {
    fn default() -> Self {
        Self {
            top_k: 4,
            snippet_chars: 400,
            max_chars: 1600,
            min_score: 0.35,
        }
    }
}

/// Формирует текстовый контекст из результатов семантического поиска.
///
/// Отбрасывает нерелевантное, обрезает фрагменты и укладывается в общий бюджет
/// символов. URI источника не включаем: модель на него не ссылается, а это
/// ~50 символов на каждый фрагмент.
pub fn build_context_string(results: &[MemoryResult], cfg: &ContextSettings) -> String {
    // Порог релевантности. Страховка: если после фильтра не осталось ничего
    // (например, апстрим не проставил score), берём исходный топ — иначе
    // поиск молча выключился бы целиком.
    let relevant: Vec<&MemoryResult> = results
        .iter()
        .filter(|r| r.score >= cfg.min_score)
        .collect();
    let picked: Vec<&MemoryResult> = if relevant.is_empty() {
        results.iter().collect()
    } else {
        relevant
    };

    let mut out = String::new();
    for (i, r) in picked.iter().take(cfg.top_k).enumerate() {
        let snippet: String = r.content.chars().take(cfg.snippet_chars).collect();
        let block = format!("[{}] {}\n", i + 1, snippet);
        // Первый блок добавляем всегда, дальше — только если влезаем в бюджет.
        if !out.is_empty() && out.len() + block.len() > cfg.max_chars {
            break;
        }
        out.push_str(&block);
    }

    if out.is_empty() {
        return "Контекст не найден в базе знаний.".to_string();
    }
    out.trim_end().to_string()
}

/// Универсальный fallback-ответ, когда LLM недоступен.
pub fn fallback_answer(profile: &AuthorProfile) -> String {
    format!(
        "К сожалению, сейчас я не могу сгенерировать ответ — сервис временно недоступен.\n\n\
         Свяжитесь со мной напрямую:\n\
         • Telegram: {tg}\n\
         • Email: {email}\n\n\
         Или оставьте заявку через форму на сайте — отвечу при наличии свободного окна.",
        tg = profile.contacts.telegram,
        email = profile.contacts.email,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::content::fallback_content;

    #[test]
    fn system_prompt_contains_author_facts() {
        let profile = fallback_content().profile;
        let prompt = chatbot_system_prompt(&profile);
        assert!(prompt.contains("Евгений Биль"));
        assert!(prompt.contains("6"));
        assert!(prompt.contains("100"));
    }

    /// Ответ начинается с сути, а не с рассказа о сайте: старый промпт требовал
    /// заканчивать каждый ответ призывом и называть цифры опыта — из-за этого
    /// ассистент повторял одно и то же на любой вопрос.
    #[test]
    fn system_prompt_requires_a_direct_answer() {
        let profile = fallback_content().profile;
        let prompt = chatbot_system_prompt(&profile);
        assert!(prompt.contains("Первое предложение"), "нет требования отвечать сразу");
        assert!(
            !prompt.contains("Каждый ответ заканчивай"),
            "вернулся обязательный призыв в конце каждого ответа"
        );
        assert!(
            prompt.contains("Не пересказывай услуги"),
            "промпт снова разрешает пересказывать услуги"
        );
    }

    /// Ни один промпт не обещает ответ «в течение 24 часов».
    ///
    /// Срок — это обязательство перед каждым, кто написал: он превращает
    /// «посмотрю, если тема моя» в «отвечу всем и вовремя». Рамка
    /// («отвечу при наличии свободного окна») снимает обязательство, не принижая
    /// автора заявки, поэтому проверка сторожит именно возврат срока.
    #[test]
    fn prompts_do_not_promise_a_reply_deadline() {
        let profile = fallback_content().profile;
        for prompt in [
            chatbot_system_prompt(&profile),
            fallback_answer(&profile),
        ] {
            let lowered = prompt.to_lowercase();
            for deadline in ["24 час", "в течение 24", "в течение дня"] {
                assert!(
                    !lowered.contains(deadline),
                    "промпт снова обещает срок «{deadline}»: {prompt}"
                );
            }
        }

        assert!(
            fallback_answer(&profile).contains("при наличии свободного окна"),
            "в fallback-ответе нет мягкой рамки"
        );
    }

    fn result(uri: &str, content: &str, score: f64) -> MemoryResult {
        MemoryResult {
            uri: uri.into(),
            content: content.into(),
            score,
        }
    }

    #[test]
    fn context_string_joins_results() {
        let results = vec![
            result("a", "hello", 0.9),
            result("b", "world", 0.8),
        ];
        let s = build_context_string(&results, &ContextSettings::default());
        assert!(s.contains("[1]"));
        assert!(s.contains("hello"));
        assert!(s.contains("world"));
        // URI больше не тратим — это чистые токены без пользы для ответа.
        assert!(!s.contains("URI"));
    }

    #[test]
    fn context_drops_irrelevant_results() {
        let results = vec![
            result("a", "по теме", 0.9),
            result("b", "шум", 0.05),
        ];
        let s = build_context_string(&results, &ContextSettings::default());
        assert!(s.contains("по теме"));
        assert!(!s.contains("шум"));
    }

    #[test]
    fn context_keeps_top_when_all_scores_are_missing() {
        // score отсутствует в ответе апстрима => 0.0 у всех. Контекст не должен
        // молча пропасть из-за порога релевантности.
        let results = vec![result("a", "факт", 0.0)];
        let s = build_context_string(&results, &ContextSettings::default());
        assert!(s.contains("факт"));
    }

    #[test]
    fn context_truncates_snippets_and_respects_budget() {
        let cfg = ContextSettings {
            top_k: 5,
            snippet_chars: 10,
            max_chars: 40,
            min_score: 0.0,
        };
        let long = "x".repeat(100);
        let results: Vec<MemoryResult> = (0..5)
            .map(|i| result(&format!("u{i}"), &long, 0.9))
            .collect();
        let s = build_context_string(&results, &cfg);
        assert!(s.len() <= cfg.max_chars + 8, "бюджет превышен: {}", s.len());
        assert!(!s.contains(&"x".repeat(11)), "сниппет не обрезан");
    }

    #[test]
    fn context_reports_empty_knowledge_base() {
        let s = build_context_string(&[], &ContextSettings::default());
        assert!(s.contains("не найден"));
    }
}

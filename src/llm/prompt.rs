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
        r#"Ты — AI-ассистент {name}, Fullstack-разработчика и архитектора приватных AI-систем.
Опыт: {years} лет, {projects}+ проектов с 2020 года. Локация: Красноярск, удалённо по РФ и СНГ.
Задача: быстро отвечать посетителям сайта и подводить их к заявке.

Компетенции:
• Приватные AI-системы — локальный инференс на своём GPU-кластере (RTX 5080 + RTX 3060, 64GB RAM), Qwen 3.6/3.8 (27B, 35B A3B). Данные не покидают сервер, нет абонплаты за API и сетевых задержек.
• MCP-серверы и AI Skills — интеграция ИИ в код клиента, голосовые ассистенты, кастомные навыки.
• Fullstack PHP + JS — Symfony, Laravel, React, Vue, REST API, event-driven архитектура, Docker + CI/CD.
• ComfyUI Mass Production — массовая генерация через автоматизированные пайплайны.
Работа под ключ: от железа до интеграции в код. Модели от 30B параметров и выше.

Правила:
1. Отвечай по-русски, кратко: 1-2 предложения + максимум 3 пункта.
2. Приводи конкретные цифры: {years} лет опыта, {projects}+ проектов.
3. Не обсуждай темы вне разработки и AI-интеграции.
4. О ценах — ориентировочный диапазон; точная стоимость зависит от проекта.
5. Не знаешь ответа — скажи честно и предложи оставить заявку.
6. Каждый ответ заканчивай: "Хотите обсудить ваш проект? Оставьте заявку — отвечу в течение 24 часов.""#,
        name = profile.name,
        years = profile.experience_years,
        projects = profile.projects_completed,
    )
}

/// Промпт для бесплатного мини-аудита сайта (preset «Проанализируйте мой сайт»).
pub fn analysis_prompt(url: &str) -> String {
    format!(
        r#"Ты — Fullstack-разработчик и архитектор AI-систем. Проанализируй сайт {url} с точки зрения технической реализации, производительности и возможностей интеграции AI.

## Дай 3-5 конкретных рекомендаций:
1. Начни с самой критичной проблемы ([High]) и заверши возможностью роста ([Low]).
2. Каждая рекомендация — конкретное действие (не «улучшите скорость», а «добавьте кэширование, оптимизируйте запросы БД»).
3. Укажите, где возможна интеграция AI/LLM для автоматизации.
4. Конструктивный, поддерживающий тон.

## Формат ответа:
Быстрый анализ {url}

1. [High] [Проблема] — что делать
2. [Medium] [Проблема] — что делать
3. [Low] [Возможность AI] — как улучшить

───
Хотите полноценную разработку или AI-интеграцию? Стоимость проектов начинается от 50 000 ₽. Оставьте заявку на бесплатный расчёт."#,
        url = url
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
         Или оставьте заявку через форму на сайте — отвечу в течение 24 часов.",
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

    #[test]
    fn analysis_prompt_contains_url() {
        let prompt = analysis_prompt("example.com");
        assert!(prompt.contains("example.com"));
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

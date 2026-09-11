//! Промпты: системный промпт чата, промпт анализа сайта, fallback-ответы.
//!
//! В проде промпты загружаются из OpenViking (`viking://.../prompts/*`). Здесь —
//! встроенные шаблоны, используемые при недоступности OpenViking.

use crate::memory::{AuthorProfile, MemoryResult};

/// Собирает системный промпт чатбота из профиля автора.
pub fn chatbot_system_prompt(profile: &AuthorProfile) -> String {
    format!(
        r#"Ты — AI-ассистент {name}, специалиста по поисковой оптимизации (SEO) с {years}-летним опытом.
Твоя задача — помогать посетителям сайта быстро получить ответы и подталкивать их к оставлению заявки.

## О специалисте:
Имя: {name}
Должность: {title}
Опыт: {years} лет, {projects}+ проектов.
Ключевые компетенции: {skills}
Контакты: Telegram {tg}, Email {email}.

## Правила общения:
1. Отвечай на русском языке, кратко и по делу (1-2 предложения + максимум 3 пункта).
2. Упоминай конкретные цифры: {years} лет опыта, {projects}+ проектов.
3. Не обещай гарантированных позиций в поиске.
4. Не отвечай на вопросы вне темы SEO и продвижения сайтов.
5. Если вопрос о ценах — давай ориентировочный диапазон и уточняй, что точная стоимость зависит от проекта.
6. Если не знаешь ответа — честно скажи и предложи оставить заявку.

## Каждый ответ должен заканчиваться CTA:
"Хотите обсудить ваш проект? Оставьте заявку — отвечу в течение 24 часов.""#,
        name = profile.name,
        years = profile.experience_years,
        projects = profile.projects_completed,
        title = profile.title,
        skills = profile.skills.join(", "),
        tg = profile.contacts.telegram,
        email = profile.contacts.email,
    )
}

/// Промпт для бесплатного мини-аудита сайта (preset «Проанализируйте мой сайт»).
pub fn analysis_prompt(url: &str) -> String {
    format!(
        r#"Ты — SEO-эксперт. Проанализируй сайт {url} и дай 3-5 конкретных рекомендаций по улучшению позиций.

## Правила:
1. Начни с самой критичной проблемы ([High]) и заверши возможностью роста ([Low]).
2. Каждая рекомендация — конкретное действие (не «улучшите скорость», а «сожмите изображения в WebP и добавьте lazy loading»).
3. Конструктивный, поддерживающий тон.

## Формат ответа:
Быстрый анализ {url}

1. [High] [Проблема] — что делать
2. [Medium] [Проблема] — что делать
3. [Low] [Возможность] — что делать

───────────
Хотите полный технический аудит с детальным отчётом? Полный SEO-аудит начинается от 15 000 ₽. Оставьте заявку на бесплатный расчёт стоимости.

Если сайт недоступен — скажи об этом и порекомендуй проверить robots.txt и доступность."#,
        url = url
    )
}

/// Формирует текстовый контекст из результатов семантического поиска.
pub fn build_context_string(results: &[MemoryResult]) -> String {
    if results.is_empty() {
        return "Контекст не найден в базе знаний.".to_string();
    }

    results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let snippet: String = r.content.chars().take(500).collect();
            format!("[Результат {}] (URI: {})\n{}", i + 1, r.uri, snippet)
        })
        .collect::<Vec<_>>()
        .join("\n\n")
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
        assert!(prompt.contains("Иван Петров"));
        assert!(prompt.contains("10"));
        assert!(prompt.contains("200"));
    }

    #[test]
    fn analysis_prompt_contains_url() {
        let prompt = analysis_prompt("example.com");
        assert!(prompt.contains("example.com"));
    }

    #[test]
    fn context_string_joins_results() {
        let results = vec![
            MemoryResult {
                uri: "a".into(),
                content: "hello".into(),
                score: 0.9,
            },
            MemoryResult {
                uri: "b".into(),
                content: "world".into(),
                score: 0.8,
            },
        ];
        let s = build_context_string(&results);
        assert!(s.contains("[Результат 1]"));
        assert!(s.contains("hello"));
    }
}

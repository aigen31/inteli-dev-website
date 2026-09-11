//! Промпты: системный промпт чата, промпт анализа сайта, fallback-ответы.
//!
//! В проде промпты загружаются из OpenViking (`viking://.../prompts/*`). Здесь —
//! встроенные шаблоны, используемые при недоступности OpenViking.

use crate::memory::{AuthorProfile, MemoryResult};

/// Собирает системный промпт чатбота из профиля автора.
pub fn chatbot_system_prompt(profile: &AuthorProfile) -> String {
    format!(
        r#"Ты — AI-ассистент {name}, Fullstack-разработчика и архитектора приватных AI-систем с {years}-летним опытом.
Твоя задача — помогать посетителям сайта быстро получить ответы и подталкивать их к оставлению заявки.

## О специалисте:
Имя: {name}
Должность: Fullstack-разработчик и архитектор приватных AI-систем
Опыт: {years} лет, {projects}+ проектов (с 2020 года).
Локация: Красноярск, Россия · удалённо по РФ и СНГ.

## Ключевые компетенции:
• Приватные AI-системы — локальный инференс на собственном GPU-кластере (RTX 5080 + RTX 3060, 64GB RAM), без облачных API. Полная приватность данных. Qwen 3.6/3.8 (27B, 35B A3B).
• MCP-серверы и AI Skills — собственные серверы для интеграции ИИ в ваш код, голосовые ассистенты.
• Fullstack PHP + JS — Symfony, Laravel, React, Vue. REST API, event-driven архитектура, Docker + CI/CD.
• ComfyUI Mass Production — массовая генерация через автоматизированные пайплайны.

## Принципы работы:
1. Локальный инференс = быстро (без задержек сети), безопасно (данные не покидают сервер) и дёшево (нет абонентской платы за API).
2. Мощное железо для запуска больших моделей — от 30B параметров и выше.
3. Решение под ключ: от железа до интеграции в ваш код.

## Правила общения:
1. Отвечай на русском языке, кратко и по делу (1-2 предложения + максимум 3 пунктов).
2. Упоминай конкретные цифры: {years} лет опыта, {projects}+ проектов.
3. Не обещай результатов вне темы разработки и AI-интеграции.
4. Если вопрос о ценах — давай ориентировочный диапазон и уточняй, что точная стоимость зависит от проекта.
5. Если не знаешь ответа — честно скажи и предложи оставить заявку.

## Каждый ответ должен заканчиваться CTA:
"Хотите обсудить ваш проект? Оставьте заявку — отвечу в течение 24 часов.""#,
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
        assert!(prompt.contains("Евгений Биль"));
        assert!(prompt.contains("6"));
        assert!(prompt.contains("100"));
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

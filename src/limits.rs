//! Пользовательские лимиты: единый источник правды для UI и валидации.
//!
//! Значения приходят из секции `[limits]` в `config.toml`. И UI (`maxlength`,
//! подсказки), и серверная валидация берут их из одного места — иначе после
//! смены конфига подсказка в интерфейсе начнёт врать, а форма будет отклонять
//! то, что браузер спокойно позволял ввести.

use std::sync::OnceLock;

use serde::Deserialize;

/// Лимиты ввода и квоты, которые видит и чувствует пользователь.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Limits {
    /// Максимальная длина вопроса в чате (и в терминале на главной).
    #[serde(default = "default_chat_message_max_chars")]
    pub chat_message_max_chars: usize,
    /// Сколько AI-ответов (запросов к LLM) один посетитель получает в час.
    /// preset-ответы из контента сюда не входят — они не тратят токены.
    #[serde(default = "default_llm_requests_per_hour")]
    pub llm_requests_per_hour: u32,
    /// Максимальная длина имени в форме заявки.
    #[serde(default = "default_lead_name_max_chars")]
    pub lead_name_max_chars: usize,
    /// Максимальная длина описания проекта в форме заявки.
    #[serde(default = "default_lead_message_max_chars")]
    pub lead_message_max_chars: usize,
    /// Максимальная длина телефона в форме заявки.
    #[serde(default = "default_lead_phone_max_chars")]
    pub lead_phone_max_chars: usize,
}

fn default_chat_message_max_chars() -> usize {
    500
}
fn default_llm_requests_per_hour() -> u32 {
    3
}
fn default_lead_name_max_chars() -> usize {
    80
}
fn default_lead_message_max_chars() -> usize {
    1000
}
fn default_lead_phone_max_chars() -> usize {
    20
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            chat_message_max_chars: default_chat_message_max_chars(),
            llm_requests_per_hour: default_llm_requests_per_hour(),
            lead_name_max_chars: default_lead_name_max_chars(),
            lead_message_max_chars: default_lead_message_max_chars(),
            lead_phone_max_chars: default_lead_phone_max_chars(),
        }
    }
}

static LIMITS: OnceLock<Limits> = OnceLock::new();

impl Limits {
    /// Публикует лимиты из конфига. Вызывается один раз при старте.
    pub fn set_global(limits: Limits) {
        let _ = LIMITS.set(limits);
    }

    /// Лимиты для UI и валидации. До `set_global` — значения по умолчанию.
    pub fn get() -> Limits {
        LIMITS.get().copied().unwrap_or_default()
    }

    /// Подсказка под полем чата: сколько символов и сколько AI-ответов в час.
    pub fn chat_hint(&self) -> String {
        format!(
            "До {} {} · {} AI-{} в час",
            self.chat_message_max_chars,
            plural_ru(
                self.chat_message_max_chars as u32,
                "символ",
                "символа",
                "символов"
            ),
            self.llm_requests_per_hour,
            plural_ru(self.llm_requests_per_hour, "ответ", "ответа", "ответов"),
        )
    }
}

/// Русская плюрализация: 1 ответ, 3 ответа, 5 ответов, 11 ответов, 21 ответ.
pub fn plural_ru<'a>(n: u32, one: &'a str, few: &'a str, many: &'a str) -> &'a str {
    let n100 = n % 100;
    let n10 = n % 10;
    if (11..=14).contains(&n100) {
        many
    } else if n10 == 1 {
        one
    } else if (2..=4).contains(&n10) {
        few
    } else {
        many
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plural_follows_russian_rules() {
        assert_eq!(plural_ru(1, "ответ", "ответа", "ответов"), "ответ");
        assert_eq!(plural_ru(3, "ответ", "ответа", "ответов"), "ответа");
        assert_eq!(plural_ru(5, "ответ", "ответа", "ответов"), "ответов");
        assert_eq!(plural_ru(11, "ответ", "ответа", "ответов"), "ответов");
        assert_eq!(plural_ru(12, "ответ", "ответа", "ответов"), "ответов");
        assert_eq!(plural_ru(21, "ответ", "ответа", "ответов"), "ответ");
        assert_eq!(plural_ru(22, "ответ", "ответа", "ответов"), "ответа");
    }

    #[test]
    fn default_limits_match_documented_values() {
        let l = Limits::default();
        assert_eq!(l.chat_message_max_chars, 500);
        assert_eq!(l.llm_requests_per_hour, 3);
        assert_eq!(l.lead_message_max_chars, 1000);
    }

    #[test]
    fn chat_hint_is_readable_russian() {
        assert_eq!(
            Limits::default().chat_hint(),
            "До 500 символов · 3 AI-ответа в час"
        );
    }

    #[test]
    fn get_falls_back_to_defaults_without_init() {
        // В этом тест-бинаре set_global не вызывался.
        assert_eq!(Limits::get(), Limits::default());
    }
}

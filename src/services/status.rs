//! Статус занятости (GET /api/status, /api/status.json).
//!
//! Здесь же — **единственный** словарь состояний. Из него собираются бейдж в
//! шапке и на главной, `/api/status`, ответ чата «Когда свободны?» и текст
//! Telegram-бота. Раньше подписи жили в четырёх местах и успели разойтись —
//! тот же класс бага, что рассинхрон preset-кнопок (см. RULES/chatbot.md).

use serde::Serialize;

use crate::memory::content::Availability;
use crate::settings::SiteSettings;

/// Постоянное подлежащее бейджа.
///
/// Короткое состояние без подлежащего («ограничен») не читается: непонятно,
/// что именно ограничено — сайт, поддержка или приём заказов. Заголовок не
/// меняется от состояния к состоянию, поэтому бейдж читается как один
/// индикатор, а не как три разных текста.
pub const SUBJECT: &str = "Приём заявок";

/// Как показать одно состояние занятости.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Presentation {
    /// Постоянное подлежащее — [`SUBJECT`].
    pub subject: &'static str,
    /// Короткое состояние: «открыт» / «ограничен» / «закрыт».
    pub state: &'static str,
    /// Расшифровка одной фразой — первая строка подсказки.
    pub summary: &'static str,
}

/// Словарь состояний.
///
/// Неизвестный статус трактуем как «закрыт»: если владелец завёл новое
/// значение в базе, а словарь ещё не обновлён, безопаснее показать «не беру»,
/// чем пообещать свободные слоты.
pub fn presentation(status: &str) -> Presentation {
    match status {
        "available" => Presentation {
            subject: SUBJECT,
            state: "открыт",
            summary: "Беру новые проекты",
        },
        "busy" => Presentation {
            subject: SUBJECT,
            state: "ограничен",
            summary: "Беру не все проекты: часть времени занята",
        },
        _ => Presentation {
            subject: SUBJECT,
            state: "закрыт",
            summary: "Новые проекты не беру",
        },
    }
}

/// Строка состояния для чата, API и бота: «Приём заявок: открыт».
pub fn status_line(status: &str) -> String {
    let p = presentation(status);
    format!("{}: {}", p.subject, p.state)
}

/// Факты о занятости — по строке на факт.
pub fn facts(a: &Availability) -> Vec<String> {
    vec![
        format!(
            "В работе {} {}",
            a.current_projects,
            crate::limits::plural_ru(a.current_projects as u32, "проект", "проекта", "проектов")
        ),
        format!("Ближайший слот: {}", a.next_free_slot),
    ]
}

/// Подсказка бейджа: расшифровка состояния и факты, по строке на пункт.
///
/// Это **дополнение** к видимой надписи, а не её замена: состояние обязано
/// читаться без наведения — на телефоне hover недоступен.
pub fn tooltip(a: &Availability) -> String {
    let mut lines = vec![presentation(&a.status).summary.to_string()];
    lines.extend(facts(a));
    lines.join("\n")
}

/// Тот же текст одной строкой — для `aria-label`.
///
/// Скринридер не видит цвет точки, поэтому состояние должно быть в тексте.
pub fn aria_label(a: &Availability) -> String {
    let mut parts = vec![status_line(&a.status)];
    parts.push(presentation(&a.status).summary.to_string());
    parts.extend(facts(a));
    parts.join(". ")
}

/// Имя иконки Lucide для статуса (вместо эмодзи).
pub fn icon_name(status: &str) -> &'static str {
    match status {
        "available" => "circle-check",
        "busy" => "settings",
        _ => "check-circle",
    }
}

/// JSON-ответ статуса занятости (см. architecture.md).
#[derive(Debug, Clone, Serialize)]
pub struct StatusPayload {
    pub status: String,
    pub label: String,
    pub icon_name: String,
    pub availability_date: String,
    pub current_projects: u8,
    pub next_free_slot: String,
    pub updated_at: String,
}

/// Формирует payload из живых настроек сайта.
///
/// Источник — [`SiteSettings`], а не контент: статус меняется из Telegram-бота,
/// и в API должно уходить именно живое значение. Контент из OpenViking остаётся
/// значением по умолчанию (см. `src/settings.rs`).
pub fn status_payload() -> StatusPayload {
    let availability = SiteSettings::availability();
    StatusPayload {
        status: availability.status.clone(),
        label: status_line(&availability.status),
        icon_name: icon_name(&availability.status).to_string(),
        availability_date: availability.availability_date,
        current_projects: availability.current_projects,
        next_free_slot: availability.next_free_slot,
        updated_at: SiteSettings::updated_at(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::content::{fallback_content, SiteContent};
    use crate::settings::SiteSettings;

    fn ensure_content() {
        SiteContent::set_global(fallback_content());
    }

    #[test]
    fn status_payload_matches_content_by_default() {
        ensure_content();
        // Общий замок: `settings::tests` тоже публикует и сбрасывает глобальное
        // состояние в этом же процессе.
        let _guard = crate::settings::test_lock();
        SiteSettings::reset_for_tests();

        let p = status_payload();
        assert_eq!(p.status, "available");
        assert_eq!(p.label, "Приём заявок: открыт");
        assert_eq!(p.icon_name, "circle-check");
        assert_eq!(p.updated_at, fallback_content().status_updated_at);
    }

    #[test]
    fn status_payload_follows_live_settings() {
        ensure_content();
        let _guard = crate::settings::test_lock();
        let mut availability = fallback_content().availability;
        availability.status = "full".into();
        availability.current_projects = 5;
        availability.next_free_slot = "с 1 декабря".into();
        SiteSettings::set_global(SiteSettings {
            availability,
            updated_at: "2026-09-24T12:00:00Z".into(),
        });

        let p = status_payload();
        assert_eq!(p.status, "full");
        assert_eq!(p.label, "Приём заявок: закрыт");
        assert_eq!(p.icon_name, "check-circle");
        assert_eq!(p.current_projects, 5);
        assert_eq!(p.next_free_slot, "с 1 декабря");
        assert_eq!(p.updated_at, "2026-09-24T12:00:00Z");

        SiteSettings::reset_for_tests();
    }

    /// Каждое состояние обязано иметь осмысленную тройку «подлежащее +
    /// состояние + расшифровка»: именно она уходит в бейдж, чат, API и бота.
    #[test]
    fn every_known_status_has_a_clear_presentation() {
        for status in ["available", "busy", "full"] {
            let p = presentation(status);
            assert_eq!(p.subject, SUBJECT, "{status}: подлежащее потеряно");
            assert!(!p.state.is_empty(), "{status}: пустое состояние");
            assert!(!p.summary.is_empty(), "{status}: пустая расшифровка");
            // Короткое состояние не должно само повторять подлежащее, иначе
            // бейдж читается как «Приём заявок: приём заявок».
            assert!(
                !p.state.contains("заявк"),
                "{status}: состояние дублирует подлежащее"
            );
        }

        assert_ne!(
            presentation("available").state,
            presentation("full").state,
            "свободен и занят не могут выглядеть одинаково"
        );
    }

    /// Неизвестное состояние не должно обещать свободные слоты.
    #[test]
    fn unknown_status_falls_back_to_closed() {
        assert_eq!(presentation("vacation").state, presentation("full").state);
    }

    /// Подсказка начинается с расшифровки и содержит оба факта, а не только
    /// цвет точки.
    #[test]
    fn tooltip_carries_state_and_facts() {
        let mut a = fallback_content().availability;
        a.status = "busy".into();
        a.current_projects = 2;
        a.next_free_slot = "немедленно".into();

        let tip = tooltip(&a);
        assert!(tip.starts_with("Беру не все проекты"), "подсказка: {tip}");
        assert!(tip.contains("В работе 2 проекта"), "подсказка: {tip}");
        assert!(tip.contains("Ближайший слот: немедленно"), "подсказка: {tip}");

        let aria = aria_label(&a);
        assert!(aria.contains("Приём заявок: ограничен"), "aria: {aria}");
        assert!(!aria.contains('\n'), "aria-label должен быть одной строкой");
    }

    /// Текст бейджа, чата и API собирается из одного словаря, поэтому не может
    /// разойтись по формулировкам.
    #[test]
    fn all_surfaces_share_one_vocabulary() {
        let a = fallback_content().availability;
        let line = status_line(&a.status);
        assert!(tooltip(&a).contains(presentation(&a.status).summary));
        assert!(aria_label(&a).starts_with(&line));
        assert_eq!(
            crate::services::chat::format_availability(&a),
            format!("{line}.\n{}.", facts(&a).join(".\n")),
            "чат и бейдж разошлись по формулировкам"
        );
    }
}

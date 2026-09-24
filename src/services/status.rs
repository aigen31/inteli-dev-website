//! Статус занятости (GET /api/status, /api/status.json).

use serde::Serialize;

use crate::services::chat::{label_for_status, status_icon_name};
use crate::settings::SiteSettings;

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
        label: label_for_status(&availability.status).to_string(),
        icon_name: status_icon_name(&availability.status).to_string(),
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
        SiteSettings::reset_for_tests();

        let p = status_payload();
        assert_eq!(p.status, "available");
        assert!(!p.label.is_empty());
        assert_eq!(p.icon_name, "circle-check");
        assert_eq!(p.updated_at, fallback_content().status_updated_at);
    }

    #[test]
    fn status_payload_follows_live_settings() {
        ensure_content();
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
        assert_eq!(p.label, "полная загрузка");
        assert_eq!(p.icon_name, "check-circle");
        assert_eq!(p.current_projects, 5);
        assert_eq!(p.next_free_slot, "с 1 декабря");
        assert_eq!(p.updated_at, "2026-09-24T12:00:00Z");

        SiteSettings::reset_for_tests();
    }
}

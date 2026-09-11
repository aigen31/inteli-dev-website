//! Статус занятости (GET /api/status, /api/status.json).

use serde::Serialize;

use crate::memory::content::SiteContent;
use crate::services::chat::{label_for_status, status_icon_name};

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

/// Формирует payload из текущего контента.
pub fn status_payload() -> StatusPayload {
    let c = SiteContent::get();
    let a = &c.availability;
    StatusPayload {
        status: a.status.clone(),
        label: label_for_status(&a.status).to_string(),
        icon_name: status_icon_name(&a.status).to_string(),
        availability_date: a.availability_date.clone(),
        current_projects: a.current_projects,
        next_free_slot: a.next_free_slot.clone(),
        updated_at: c.status_updated_at.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::content::fallback_content;

    #[test]
    fn status_payload_matches_content() {
        SiteContent::set_global(fallback_content());
        let p = status_payload();
        assert_eq!(p.status, "available");
        assert!(!p.label.is_empty());
        assert_eq!(p.icon_name, "circle-check");
    }
}

//! Memory Layer — интеграция с OpenViking (`viking://` URIs).
//!
//! OpenViking — единый источник правды об авторе (профиль, услуги, проекты,
//! статус занятости). При недоступности OpenViking приложение деградирует на
//! встроенный fallback-контент ([`content::fallback_content`]) — см. `memory-storage.md`.

pub mod client;
pub mod content;

pub use client::{MemoryResult, OpenVikingClient};
pub use content::{AuthorProfile, Availability};

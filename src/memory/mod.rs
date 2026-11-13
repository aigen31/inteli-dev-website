//! Контент сайта: профиль автора, услуги, проекты, статус занятости.
//!
//! Источник — встроенные значения ([`content::builtin_content`]) и страницы
//! услуг из [`service_pages`]. Живые настройки (статус занятости, прайс) лежат
//! в SQLite и правятся из админки или Telegram-бота.

pub mod content;
pub mod service_pages;

pub use content::{AuthorProfile, Availability};

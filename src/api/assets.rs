//! Встроенные статические ресурсы (CSS/JS/robots/manifest/favicon).
//!
//! Используем `include_str!`, чтобы бинарник был самодостаточным и Docker-образ
//! не зависел от файловой системы (P2: лёгкость — это фича).
//!
//! Карты сайта здесь нет: она строится на лету
//! ([`crate::services::feed::current_sitemap`]), потому что должна включать
//! опубликованные статьи.

pub const STYLE_CSS: &str = include_str!("../../assets/style.css");
pub const MAIN_JS: &str = include_str!("../../assets/main.js");
pub const ROBOTS_TXT: &str = include_str!("../../assets/robots.txt");
pub const MANIFEST_JSON: &str = include_str!("../../assets/manifest.json");
pub const FAVICON_SVG: &str = include_str!("../../assets/favicon.svg");

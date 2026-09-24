//! Встроенные статические ресурсы (CSS/JS/manifest/favicon).
//!
//! Используем `include_str!`, чтобы бинарник был самодостаточным и Docker-образ
//! не зависел от файловой системы (P2: лёгкость — это фича).
//!
//! Здесь нет ни карты сайта, ни robots.txt: оба зависят от публичного адреса
//! сайта. Карта строится на лету ([`crate::services::feed::current_sitemap`]),
//! потому что включает опубликованные статьи; robots — тоже
//! ([`crate::services::seo::robots_txt`]), потому что адрес карты сайта в нём
//! обязан совпадать с `PUBLIC_URL`.

pub const STYLE_CSS: &str = include_str!("../../assets/style.css");
pub const MAIN_JS: &str = include_str!("../../assets/main.js");
pub const MANIFEST_JSON: &str = include_str!("../../assets/manifest.json");
pub const FAVICON_SVG: &str = include_str!("../../assets/favicon.svg");

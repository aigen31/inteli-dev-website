//! Встроенные статические ресурсы (CSS/JS/robots/manifest/sitemap/favicon).
//!
//! Используем `include_str!`, чтобы бинарник был самодостаточным и Docker-образ
//! не зависел от файловой системы (P2: лёгкость — это фича).

pub const STYLE_CSS: &str = include_str!("../../assets/style.css");
pub const MAIN_JS: &str = include_str!("../../assets/main.js");
pub const ROBOTS_TXT: &str = include_str!("../../assets/robots.txt");
pub const MANIFEST_JSON: &str = include_str!("../../assets/manifest.json");
pub const SITEMAP_XML: &str = include_str!("../../assets/sitemap.xml");
pub const FAVICON_SVG: &str = include_str!("../../assets/favicon.svg");

//! Встроенные статические ресурсы (CSS/JS/manifest/favicon/og-обложка).
//!
//! Используем `include_str!`/`include_bytes!`, чтобы бинарник был
//! самодостаточным и Docker-образ не зависел от файловой системы (P2: лёгкость —
//! это фича).
//!
//! Здесь нет ни карты сайта, ни robots.txt: оба зависят от публичного адреса
//! сайта. Карта строится на лету ([`crate::services::feed::current_sitemap`]),
//! потому что включает опубликованные статьи; robots — тоже
//! ([`crate::services::seo::robots_txt`]), потому что адрес карты сайта в нём
//! обязан совпадать с `PUBLIC_URL`.
//!
//! # Версия ресурсов в адресе
//!
//! CSS и JS подключены без отпечатка в имени (`/assets/style.css`), поэтому
//! «кэшировать на год» для них нельзя: после правки браузер покажет старую
//! версию. Отсюда [`versioned`]: адрес получает `?v=<хеш содержимого>`, и
//! версия меняется ровно тогда, когда меняется файл. Хеш считается по
//! содержимому встроенных ресурсов, а не берётся из `APP_VERSION`, — иначе
//! забытая правка `APP_VERSION` снова дала бы несвежий CSS.

use std::sync::OnceLock;

use sha2::{Digest, Sha256};

pub const STYLE_CSS: &str = include_str!("../../assets/style.css");
pub const MAIN_JS: &str = include_str!("../../assets/main.js");
pub const MANIFEST_JSON: &str = include_str!("../../assets/manifest.json");
pub const FAVICON_SVG: &str = include_str!("../../assets/favicon.svg");

/// Обложка для соцсетей (`og:image`), 1200×630.
///
/// Исходник — `assets/og-default.svg`, пересборка — `scripts/make-og-image.sh`.
/// В разметку уходит растровый PNG: SVG не понимают ни Telegram, ни VK.
pub const OG_IMAGE: &[u8] = include_bytes!("../../assets/og-default.png");

/// Путь, по которому отдаётся [`OG_IMAGE`].
pub const OG_IMAGE_PATH: &str = "/og.png";

/// Кэш версионированного ресурса: содержимое неизменяемо в пределах адреса.
pub const CACHE_IMMUTABLE: &str = "public, max-age=31536000, immutable";

/// Кэш ресурса без версии: правка должна доехать за час, а не через год.
pub const CACHE_SHORT: &str = "public, max-age=3600";

/// Короткий хеш содержимого встроенных ресурсов.
///
/// Восемь шестнадцатеричных символов — как у git: их достаточно, чтобы версии
/// не совпали, и они не удлиняют адрес.
pub fn version() -> &'static str {
    static VERSION: OnceLock<String> = OnceLock::new();

    VERSION.get_or_init(|| {
        let mut hasher = Sha256::new();
        for part in [STYLE_CSS, MAIN_JS, MANIFEST_JSON, FAVICON_SVG] {
            hasher.update(part.as_bytes());
        }
        hasher.update(OG_IMAGE);

        hex::encode(hasher.finalize())[..8].to_string()
    })
}

/// Адрес встроенного ресурса с версией: `/assets/style.css?v=1a2b3c4d`.
pub fn versioned(path: &str) -> String {
    format!("{path}?v={}", version())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_stable_and_short() {
        let first = version();
        assert_eq!(first, version(), "версия не должна меняться в процессе");
        assert_eq!(first.len(), 8);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn versioned_url_carries_the_version() {
        assert_eq!(
            versioned("/assets/style.css"),
            format!("/assets/style.css?v={}", version())
        );
    }
}

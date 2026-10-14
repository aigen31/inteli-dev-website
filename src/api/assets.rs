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

/// Знак Habr (`/assets/habr.svg`) — ссылка на профиль в шапке.
///
/// Отдельный файл, а не инлайн-`<svg>` в разметке: официальный знак — сложная
/// фигура на ~16 КБ, и в каждой HTML-странице она удваивала бы вес документа.
/// Здесь же он уходит одним запросом с версией в адресе и живёт в кэше год.
/// Заливка в файле белая (в `<img>` `currentColor` не работает), яркость
/// приглушает CSS.
pub const HABR_SVG: &str = include_str!("../../assets/habr.svg");

/// Путь, по которому отдаётся [`HABR_SVG`].
pub const HABR_ICON_PATH: &str = "/assets/habr.svg";

/// Обложка для соцсетей (`og:image`), 1200×630.
///
/// Исходник — `assets/og-default.svg`, пересборка — `scripts/make-og-image.sh`.
/// В разметку уходит растровый PNG: SVG не понимают ни Telegram, ни VK.
pub const OG_IMAGE: &[u8] = include_bytes!("../../assets/og-default.png");

/// Путь, по которому отдаётся [`OG_IMAGE`].
pub const OG_IMAGE_PATH: &str = "/og.png";

/// Один кадр маскота: эмоция и её картинка.
///
/// Кадры — одна и та же поза на одном холсте, поэтому при смене эмоции ведьма
/// не дёргается. Размер холста (144×192) — свойство файлов, а
/// не догадка разметки: его считает `scripts/make-mascot.mjs`, а здесь он лежит
/// рядом с картинками, чтобы `width`/`height` в разметке брались из одного
/// места и браузер резервировал прямоугольник до загрузки.
pub struct MascotFrame {
    /// Имя эмоции — оно же последний сегмент адреса (`/assets/mascot/base.png`).
    pub emotion: &'static str,
    /// Содержимое PNG.
    pub image: &'static [u8],
}

/// Ширина кадра маскота в логических пикселях.
pub const MASCOT_WIDTH: u32 = 144;

/// Высота кадра маскота в логических пикселях.
///
/// 144×192 — холст-источник. CSS показывает его в 144, 120 или 96px по ширине:
/// пропорции 3:4 сохранены, поэтому кадр всегда влезает в свою полосу между
/// чатом и описанием и не требует обрезки по высоте.
pub const MASCOT_HEIGHT: u32 = 192;

/// Маскот сайта — ведьма в полосе между AI-терминалом и описанием в hero.
///
/// Кадр на эмоцию: простой, запрос в полёте, ответ печатается, ответ допечатан
/// и неудача. Разметка выводит все пять сразу, а переключает их `data-emotion`
/// на общем контейнере — так смена эмоции не ждёт сети и не мигает пустотой.
/// Порядок кадров произвольный: ищут их по имени, а не по индексу.
pub const MASCOT_FRAMES: [MascotFrame; 5] = [
    MascotFrame {
        emotion: "base",
        image: include_bytes!("../../assets/mascot/base.png"),
    },
    MascotFrame {
        emotion: "thinking",
        image: include_bytes!("../../assets/mascot/thinking.png"),
    },
    MascotFrame {
        emotion: "read",
        image: include_bytes!("../../assets/mascot/read.png"),
    },
    MascotFrame {
        emotion: "smirk",
        image: include_bytes!("../../assets/mascot/smirk.png"),
    },
    MascotFrame {
        emotion: "dissatisfied",
        image: include_bytes!("../../assets/mascot/dissatisfied.png"),
    },
];

/// Префикс адресов кадров маскота: `/assets/mascot/<эмоция>.png`.
pub const MASCOT_PATH_PREFIX: &str = "/assets/mascot/";

/// Адрес кадра маскота по имени эмоции.
pub fn mascot_path(emotion: &str) -> String {
    format!("{MASCOT_PATH_PREFIX}{emotion}.png")
}

/// Картинка кадра по имени эмоции: `None` — такой эмоции нет.
pub fn mascot_frame(emotion: &str) -> Option<&'static [u8]> {
    MASCOT_FRAMES
        .iter()
        .find(|frame| frame.emotion == emotion)
        .map(|frame| frame.image)
}

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
        for part in [STYLE_CSS, MAIN_JS, MANIFEST_JSON, FAVICON_SVG, HABR_SVG] {
            hasher.update(part.as_bytes());
        }
        hasher.update(OG_IMAGE);
        for frame in &MASCOT_FRAMES {
            hasher.update(frame.image);
        }

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

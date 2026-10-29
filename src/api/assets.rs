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

/// Картинка кейса из портфолио: файл, содержимое и реальные размеры.
///
/// Скриншоты проектов лежат рядом с кодом, а не в базе медиа: портфолио — часть
/// контента сайта, оно меняется вместе с ним, и `include_bytes!` не даёт
/// картинке потеряться при переезде базы. `width`/`height` держим здесь же,
/// чтобы браузер резервировал место до загрузки и карточки не прыгали.
pub struct ProjectImage {
    /// Имя файла — оно же последний сегмент адреса (`/assets/projects/x.webp`).
    pub file: &'static str,
    /// Содержимое WebP.
    pub image: &'static [u8],
    pub width: u32,
    pub height: u32,
}

/// Скриншоты кейсов. Пересобираются из портфолио: исходники — PNG/JPEG 1920×1080,
/// на сайте — WebP шириной до 1600 px (см. `RULES/ui-rules.md`).
pub const PROJECT_IMAGES: [ProjectImage; 8] = [
    ProjectImage {
        file: "voice-ai-real-estate.webp",
        image: include_bytes!("../../assets/projects/voice-ai-real-estate.webp"),
        width: 1600,
        height: 896,
    },
    ProjectImage {
        file: "ai-browser-agent.webp",
        image: include_bytes!("../../assets/projects/ai-browser-agent.webp"),
        width: 1600,
        height: 900,
    },
    ProjectImage {
        file: "coffee-b2b-platform.webp",
        image: include_bytes!("../../assets/projects/coffee-b2b-platform.webp"),
        width: 1600,
        height: 900,
    },
    ProjectImage {
        file: "pdd-pro-maintenance.webp",
        image: include_bytes!("../../assets/projects/pdd-pro-maintenance.webp"),
        width: 1600,
        height: 900,
    },
    ProjectImage {
        file: "wordpress-devops-refactor.webp",
        image: include_bytes!("../../assets/projects/wordpress-devops-refactor.webp"),
        width: 1600,
        height: 900,
    },
    ProjectImage {
        file: "symfony-kimai-custom.webp",
        image: include_bytes!("../../assets/projects/symfony-kimai-custom.webp"),
        width: 1600,
        height: 900,
    },
    ProjectImage {
        file: "edu-platform-marketplace.webp",
        image: include_bytes!("../../assets/projects/edu-platform-marketplace.webp"),
        width: 1600,
        height: 900,
    },
    ProjectImage {
        file: "wordpress-woocommerce-plugins.webp",
        image: include_bytes!("../../assets/projects/wordpress-woocommerce-plugins.webp"),
        width: 1200,
        height: 800,
    },
];

/// Шрифт, встроенный в бинарник.
///
/// Inter и JetBrains Mono — вариативные (`wght 100–900`), с кириллицей. Раньше
/// сайт полагался на системный `Inter`, и там, где его нет, весь набор съезжал
/// на `system-ui`: заголовки теряли плотность, а моно-подписи — характер.
/// Файлы отдаются с того же origin: `font-src 'self'` в CSP внешние шрифты
/// запрещает.
pub struct FontFile {
    /// Имя файла в адресе `/assets/fonts/<файл>`.
    pub file: &'static str,
    /// Содержимое woff2.
    pub bytes: &'static [u8],
}

pub static FONT_FILES: &[FontFile] = &[
    FontFile {
        file: "inter-cyrillic-wght-normal.woff2",
        bytes: include_bytes!("../../assets/fonts/inter-cyrillic-wght-normal.woff2"),
    },
    FontFile {
        file: "inter-cyrillic-ext-wght-normal.woff2",
        bytes: include_bytes!("../../assets/fonts/inter-cyrillic-ext-wght-normal.woff2"),
    },
    FontFile {
        file: "inter-latin-wght-normal.woff2",
        bytes: include_bytes!("../../assets/fonts/inter-latin-wght-normal.woff2"),
    },
    FontFile {
        file: "inter-latin-ext-wght-normal.woff2",
        bytes: include_bytes!("../../assets/fonts/inter-latin-ext-wght-normal.woff2"),
    },
    FontFile {
        file: "jetbrains-mono-cyrillic-wght-normal.woff2",
        bytes: include_bytes!("../../assets/fonts/jetbrains-mono-cyrillic-wght-normal.woff2"),
    },
    FontFile {
        file: "jetbrains-mono-cyrillic-ext-wght-normal.woff2",
        bytes: include_bytes!(
            "../../assets/fonts/jetbrains-mono-cyrillic-ext-wght-normal.woff2"
        ),
    },
    FontFile {
        file: "jetbrains-mono-latin-wght-normal.woff2",
        bytes: include_bytes!("../../assets/fonts/jetbrains-mono-latin-wght-normal.woff2"),
    },
];

/// Шрифт по имени файла: `None` — такого файла нет.
pub fn font_file(file: &str) -> Option<&'static FontFile> {
    FONT_FILES.iter().find(|font| font.file == file)
}

/// Префикс адресов шрифтов: `/assets/fonts/<файл>`.
pub const FONT_PATH_PREFIX: &str = "/assets/fonts/";

/// Иллюстрация hero — «левитация» Кики, 2560×1440 WebP.
///
/// Сгенерирована локально (ComfyUI, Qwen-Image 2.1), исходник лежит в
/// `assets/hero/`. Встроена в бинарник по той же причине, что и скриншоты
/// кейсов: страница не должна зависеть от файловой системы контейнера.
pub const HERO_ART: &[u8] = include_bytes!("../../assets/hero/hero-kiki-levitation-16x9.webp");
/// Адрес иллюстрации hero.
pub const HERO_ART_PATH: &str = "/assets/hero/kiki-levitation.webp";
/// Ширина иллюстрации hero в пикселях.
pub const HERO_ART_WIDTH: u32 = 2560;
/// Высота иллюстрации hero в пикселях.
pub const HERO_ART_HEIGHT: u32 = 1440;

/// Логотип и иконки приложения.
///
/// Знак — шляпа Кики, сгенерирована в той же one-line манере, что и hero-арт,
/// и переведена в вектор: контур трассирован по генерации. Правило размеров:
/// до 48 px — белая заливка (контур там исчезает), от 180 px — тонкий контур.
pub struct LogoFile {
    /// Имя файла в адресе `/assets/logo/<файл>`.
    pub file: &'static str,
    /// Содержимое файла.
    pub bytes: &'static [u8],
    /// MIME-тип: PNG для иконок, SVG для знака в интерфейсе.
    pub mime: &'static str,
}

pub static LOGO_FILES: &[LogoFile] = &[
    LogoFile {
        file: "hat-icon-fill-16.png",
        bytes: include_bytes!("../../assets/logo/hat-icon-fill-16.png"),
        mime: "image/png",
    },
    LogoFile {
        file: "hat-icon-fill-32.png",
        bytes: include_bytes!("../../assets/logo/hat-icon-fill-32.png"),
        mime: "image/png",
    },
    LogoFile {
        file: "hat-icon-fill-48.png",
        bytes: include_bytes!("../../assets/logo/hat-icon-fill-48.png"),
        mime: "image/png",
    },
    LogoFile {
        file: "hat-icon-line-180.png",
        bytes: include_bytes!("../../assets/logo/hat-icon-line-180.png"),
        mime: "image/png",
    },
    LogoFile {
        file: "hat-icon-line-192.png",
        bytes: include_bytes!("../../assets/logo/hat-icon-line-192.png"),
        mime: "image/png",
    },
    LogoFile {
        file: "hat-icon-line-512.png",
        bytes: include_bytes!("../../assets/logo/hat-icon-line-512.png"),
        mime: "image/png",
    },
    LogoFile {
        file: "hat-fill.svg",
        bytes: include_bytes!("../../assets/logo/hat-fill-light.svg"),
        mime: "image/svg+xml",
    },
    LogoFile {
        file: "hat-line.svg",
        bytes: include_bytes!("../../assets/logo/hat-line-thin-light.svg"),
        mime: "image/svg+xml",
    },
];

/// Логотип по имени файла: `None` — такого файла нет.
pub fn logo_file(file: &str) -> Option<&'static LogoFile> {
    LOGO_FILES.iter().find(|logo| logo.file == file)
}

/// Префикс адресов логотипа: `/assets/logo/<файл>`.
pub const LOGO_PATH_PREFIX: &str = "/assets/logo/";

/// Favicon в SVG: белая заливка на тёмном тайле.
///
/// Отдельно от [`LOGO_FILES`], потому что адрес у него корневой — `/favicon.svg`,
/// и на него рассчитывают браузеры, а не разметка сайта.
pub const FAVICON_SVG: &str = include_str!("../../assets/favicon.svg");

/// Префикс адресов скриншотов кейсов: `/assets/projects/<файл>`.
pub const PROJECT_IMAGE_PATH_PREFIX: &str = "/assets/projects/";


/// Иллюстрация к услуге: файл, содержимое и размеры.
///
/// Кики для карточек услуг: тот же персонаж, что в hero, но в другой роли —
/// у каждой услуги свой предмет. Файлы лежат рядом с кодом и вшиваются в
/// бинарник: карточка услуги не должна зависеть от базы медиа.
pub struct ServiceArt {
    /// Слаг услуги — он же имя файла без расширения.
    pub slug: &'static str,
    pub image: &'static [u8],
    pub width: u32,
    pub height: u32,
}

/// Префикс адреса иллюстраций услуг.
pub const SERVICE_ART_PATH_PREFIX: &str = "/assets/services/";

/// Все иллюстрации услуг.
pub const SERVICE_ART: [ServiceArt; 19] = [
    ServiceArt {
        slug: "ai-assistants",
        image: include_bytes!("../../assets/services/ai-assistants.png"),
        width: 760,
        height: 680,
    },
    ServiceArt {
        slug: "ai-automation",
        image: include_bytes!("../../assets/services/ai-automation.png"),
        width: 760,
        height: 760,
    },
    ServiceArt {
        slug: "ai-bots",
        image: include_bytes!("../../assets/services/ai-bots.png"),
        width: 760,
        height: 760,
    },
    ServiceArt {
        slug: "ai-mcp",
        image: include_bytes!("../../assets/services/ai-mcp.png"),
        width: 676,
        height: 760,
    },
    ServiceArt {
        slug: "ai-vision",
        image: include_bytes!("../../assets/services/ai-vision.png"),
        width: 744,
        height: 760,
    },
    ServiceArt {
        slug: "comfyui-media",
        image: include_bytes!("../../assets/services/comfyui-media.png"),
        width: 760,
        height: 760,
    },
    ServiceArt {
        slug: "design-figma",
        image: include_bytes!("../../assets/services/design-figma.png"),
        width: 710,
        height: 760,
    },
    ServiceArt {
        slug: "desktop-apps",
        image: include_bytes!("../../assets/services/desktop-apps.png"),
        width: 760,
        height: 760,
    },
    ServiceArt {
        slug: "email-campaigns",
        image: include_bytes!("../../assets/services/email-campaigns.png"),
        width: 760,
        height: 760,
    },
    ServiceArt {
        slug: "fullstack-any",
        image: include_bytes!("../../assets/services/fullstack-any.png"),
        width: 744,
        height: 760,
    },
    ServiceArt {
        slug: "legacy-refactor",
        image: include_bytes!("../../assets/services/legacy-refactor.png"),
        width: 760,
        height: 751,
    },
    ServiceArt {
        slug: "local-private-dev",
        image: include_bytes!("../../assets/services/local-private-dev.png"),
        width: 760,
        height: 760,
    },
    ServiceArt {
        slug: "mobile-apps",
        image: include_bytes!("../../assets/services/mobile-apps.png"),
        width: 760,
        height: 760,
    },
    ServiceArt {
        slug: "qa-testing",
        image: include_bytes!("../../assets/services/qa-testing.png"),
        width: 760,
        height: 760,
    },
    ServiceArt {
        slug: "small-fixes",
        image: include_bytes!("../../assets/services/small-fixes.png"),
        width: 708,
        height: 760,
    },
    ServiceArt {
        slug: "support",
        image: include_bytes!("../../assets/services/support.png"),
        width: 760,
        height: 760,
    },
    ServiceArt {
        slug: "sysadmin-devops",
        image: include_bytes!("../../assets/services/sysadmin-devops.png"),
        width: 760,
        height: 760,
    },
    ServiceArt {
        slug: "voice-ai",
        image: include_bytes!("../../assets/services/voice-ai.png"),
        width: 643,
        height: 760,
    },
    ServiceArt {
        slug: "web-any",
        image: include_bytes!("../../assets/services/web-any.png"),
        width: 760,
        height: 760,
    },
];

/// Адрес иллюстрации услуги.
pub fn service_art_path(slug: &str) -> String {
    format!("{SERVICE_ART_PATH_PREFIX}{slug}.png")
}

/// Иллюстрация по слагу услуги: `None` — для этой услуги рисунка нет.
pub fn service_art(slug: &str) -> Option<&'static ServiceArt> {
    SERVICE_ART.iter().find(|art| art.slug == slug)
}

/// Адрес скриншота кейса по имени файла.
pub fn project_image_path(file: &str) -> String {
    format!("{PROJECT_IMAGE_PATH_PREFIX}{file}")
}

/// Скриншот кейса по имени файла: `None` — такого файла нет.
pub fn project_image(file: &str) -> Option<&'static ProjectImage> {
    PROJECT_IMAGES.iter().find(|image| image.file == file)
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
        for image in &PROJECT_IMAGES {
            hasher.update(image.image);
        }
        // Иллюстрации услуг меняются чаще остального, а кэш у них годовой.
        // Без них версия адреса не двигалась, и браузер показывал старые
        // картинки после каждой новой генерации.
        for art in &SERVICE_ART {
            hasher.update(art.image);
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

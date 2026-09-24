//! Изменяемые настройки сайта — то, что владелец правит на ходу.
//!
//! # Зачем отдельный слой
//!
//! `SiteContent` — это контент из OpenViking (профиль, услуги, проекты). Он
//! загружается один раз при старте и дальше неизменяем. Статус занятости
//! устроен иначе: это операционное состояние, которое меняется несколько раз в
//! день, и менять его нужно быстро — из Telegram, с телефона, без пересборки и
//! перезапуска.
//!
//! Поэтому живой статус живёт здесь, в [`SiteSettings`], а
//! [`crate::memory::content::SiteContent::availability`] остаётся значением по
//! умолчанию: им пользуются, пока в БД нет сохранённого значения.
//!
//! # Почему SQLite, а не OpenViking
//!
//! OpenViking — источник правды для *контента* (кто я, что делаю, сколько
//! стоит). Статус занятости — это *состояние сервиса*: он должен переживать
//! рестарт и быть записан даже когда OpenViking недоступен. Писать его в
//! SQLite рядом с заявками надёжнее и не требует сети.

use std::sync::RwLock;

use crate::memory::content::{Availability, SiteContent};

/// Живые настройки сайта.
#[derive(Debug, Clone, PartialEq)]
pub struct SiteSettings {
    pub availability: Availability,
    /// Когда статус менялся в последний раз (RFC3339).
    pub updated_at: String,
}

static SETTINGS: RwLock<Option<SiteSettings>> = RwLock::new(None);

impl SiteSettings {
    /// Публикует настройки. Вызывается при старте (после загрузки из БД) и
    /// после каждого изменения из Telegram-бота.
    pub fn set_global(settings: SiteSettings) {
        let mut guard = SETTINGS.write().unwrap_or_else(|e| e.into_inner());
        *guard = Some(settings);
    }

    /// Текущие настройки. `None` — [`set_global`](Self::set_global) ещё не
    /// вызывался (например, в юнит-тестах модулей ниже по стеку).
    pub fn get() -> Option<SiteSettings> {
        let guard = SETTINGS.read().unwrap_or_else(|e| e.into_inner());
        guard.clone()
    }

    /// Живой статус занятости.
    ///
    /// Пока настройки не опубликованы, отдаём значение из контента — так
    /// страницы и `/api/status` работают до первого изменения и в тестах, где
    /// `SiteSettings` не инициализируется вовсе.
    pub fn availability() -> Availability {
        match Self::get() {
            Some(s) => s.availability,
            None => SiteContent::get().availability.clone(),
        }
    }

    /// Когда статус менялся в последний раз: из настроек, иначе из контента.
    pub fn updated_at() -> String {
        match Self::get() {
            Some(s) => s.updated_at,
            None => SiteContent::get().status_updated_at.clone(),
        }
    }

    /// Сбрасывает глобальное состояние.
    ///
    /// Нужен только тестам: они делят один процесс, и статус, выставленный
    /// одним тестом, не должен утекать в другой. В продакшене состояние
    /// публикуется один раз при старте и дальше только заменяется.
    #[doc(hidden)]
    pub fn reset_global() {
        let mut guard = SETTINGS.write().unwrap_or_else(|e| e.into_inner());
        *guard = None;
    }

    /// Сбрасывает глобальное состояние. Только для тестов: они делят процесс,
    /// и один тест не должен видеть настройки, опубликованные другим.
    #[cfg(test)]
    pub fn reset_for_tests() {
        Self::reset_global()
    }
}

/// Строит настройки по умолчанию из контента (первый запуск, нет записи в БД).
pub fn defaults_from_content() -> SiteSettings {
    let content = SiteContent::get();
    SiteSettings {
        availability: content.availability.clone(),
        updated_at: content.status_updated_at.clone(),
    }
}

/// Читает настройки для работы: сохранённые в БД либо значения по умолчанию.
///
/// Читаем из БД, а не из глобального состояния, чтобы ответ на команду в
/// Telegram опирался на актуальную запись, даже если процесс только что
/// поднялся или настройки правились из другого места.
pub async fn load_or_default(pool: &sqlx::SqlitePool) -> crate::error::AppResult<SiteSettings> {
    match crate::storage::settings::load_availability(pool).await? {
        Some((availability, updated_at)) => Ok(SiteSettings {
            availability,
            updated_at,
        }),
        None => Ok(defaults_from_content()),
    }
}

/// Сохраняет статус занятости и сразу публикует его глобально.
///
/// Единственная точка записи: и Telegram-бот, и любой будущий вызывающий
/// получают одинаковый результат — данные в БД и живое состояние сайта
/// не могут разойтись.
pub async fn update_availability(
    pool: &sqlx::SqlitePool,
    availability: Availability,
) -> crate::error::AppResult<SiteSettings> {
    let updated_at = chrono::Utc::now().to_rfc3339();
    crate::storage::settings::save_availability(pool, &availability, &updated_at).await?;

    let settings = SiteSettings {
        availability,
        updated_at,
    };
    SiteSettings::set_global(settings.clone());
    tracing::info!(
        "настройки сайта обновлены: статус = {}",
        settings.availability.status
    );
    Ok(settings)
}

/// Общий замок для тестов, работающих с глобальным [`SiteSettings`].
///
/// Тесты одного бинарника идут параллельно и делят процесс. Сценарий
/// «выставить статус → прочитать» без общего замка — гонка: соседний тест
/// успевает сбросить настройки между двумя вызовами, и проверка падает
/// случайным образом (а не по делу).
#[cfg(test)]
pub fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    // Отравление возможно только при панике внутри самих тестов — не повод
    // ронять остальные.
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::content::fallback_content;

    fn ensure_content() {
        // SiteContent инициализируется один раз на процесс — повторный вызов
        // безопасен (OnceLock::set игнорирует ошибку).
        SiteContent::set_global(fallback_content());
    }

    #[test]
    fn falls_back_to_content_when_not_published() {
        ensure_content();
        let _guard = test_lock();
        SiteSettings::reset_for_tests();

        let a = SiteSettings::availability();
        assert_eq!(a.status, "available");
        assert_eq!(SiteSettings::updated_at(), fallback_content().status_updated_at);
    }

    #[test]
    fn published_settings_win_over_content() {
        ensure_content();
        let _guard = test_lock();
        let mut availability = fallback_content().availability;
        availability.status = "full".into();
        availability.current_projects = 4;
        SiteSettings::set_global(SiteSettings {
            availability: availability.clone(),
            updated_at: "2026-09-24T12:00:00Z".into(),
        });

        assert_eq!(SiteSettings::availability(), availability);
        assert_eq!(SiteSettings::updated_at(), "2026-09-24T12:00:00Z");

        SiteSettings::reset_for_tests();
    }

    #[test]
    fn defaults_from_content_mirror_the_content() {
        ensure_content();
        let _guard = test_lock();
        let defaults = defaults_from_content();
        assert_eq!(defaults.availability, fallback_content().availability);
    }
}

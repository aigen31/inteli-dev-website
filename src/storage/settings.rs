//! Репозиторий настроек сайта (таблица `site_settings`).
//!
//! Хранилище — «ключ → JSON»: сейчас там одна запись `availability`, но схема
//! не требует миграции под каждую новую настройку, которую владелец захочет
//! менять из Telegram.

use sqlx::sqlite::SqlitePool;

use crate::error::AppResult;
use crate::memory::content::Availability;

/// Ключ записи со статусом занятости.
pub const AVAILABILITY_KEY: &str = "availability";

/// Загружает сохранённый статус занятости вместе с временем изменения.
///
/// `Ok(None)` — в БД ещё ничего не сохраняли (первый запуск): вызывающий
/// использует значения по умолчанию из контента.
pub async fn load_availability(pool: &SqlitePool) -> AppResult<Option<(Availability, String)>> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT value, updated_at FROM site_settings WHERE key = ?")
            .bind(AVAILABILITY_KEY)
            .fetch_optional(pool)
            .await?;

    let Some((value, updated_at)) = row else {
        return Ok(None);
    };

    // Битый JSON не должен ронять старт приложения: логируем и откатываемся
    // на значения по умолчанию из контента.
    match serde_json::from_str::<Availability>(&value) {
        Ok(availability) => Ok(Some((availability, updated_at))),
        Err(e) => {
            tracing::warn!("site_settings.{AVAILABILITY_KEY}: повреждённый JSON, игнорируем: {e}");
            Ok(None)
        }
    }
}

/// Сохраняет статус занятости (upsert) и возвращает время записи.
pub async fn save_availability(
    pool: &SqlitePool,
    availability: &Availability,
    updated_at: &str,
) -> AppResult<()> {
    let value = serde_json::to_string(availability)?;
    sqlx::query(
        r#"INSERT INTO site_settings (key, value, updated_at) VALUES (?, ?, ?)
           ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at"#,
    )
    .bind(AVAILABILITY_KEY)
    .bind(value)
    .bind(updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::content::fallback_content;
    use crate::storage::init_memory_pool;

    #[tokio::test]
    async fn missing_row_returns_none() {
        let pool = init_memory_pool().await.unwrap();
        assert!(load_availability(&pool).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn save_then_load_roundtrips() {
        let pool = init_memory_pool().await.unwrap();
        let mut availability = fallback_content().availability;
        availability.status = "busy".into();
        availability.current_projects = 3;
        availability.next_free_slot = "с 1 октября".into();

        save_availability(&pool, &availability, "2026-09-24T12:00:00Z")
            .await
            .unwrap();

        let (loaded, updated_at) = load_availability(&pool).await.unwrap().unwrap();
        assert_eq!(loaded, availability);
        assert_eq!(updated_at, "2026-09-24T12:00:00Z");
    }

    #[tokio::test]
    async fn save_overwrites_previous_value() {
        let pool = init_memory_pool().await.unwrap();
        let mut availability = fallback_content().availability;

        availability.status = "busy".into();
        save_availability(&pool, &availability, "2026-09-24T12:00:00Z")
            .await
            .unwrap();

        availability.status = "full".into();
        save_availability(&pool, &availability, "2026-09-24T13:00:00Z")
            .await
            .unwrap();

        let (loaded, updated_at) = load_availability(&pool).await.unwrap().unwrap();
        assert_eq!(loaded.status, "full");
        assert_eq!(updated_at, "2026-09-24T13:00:00Z");

        // Строка ровно одна — upsert, а не накопление истории.
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM site_settings")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn corrupted_json_is_ignored_not_fatal() {
        let pool = init_memory_pool().await.unwrap();
        sqlx::query("INSERT INTO site_settings (key, value, updated_at) VALUES (?, ?, ?)")
            .bind(AVAILABILITY_KEY)
            .bind("{not json")
            .bind("2026-09-24T12:00:00Z")
            .execute(&pool)
            .await
            .unwrap();

        assert!(load_availability(&pool).await.unwrap().is_none());
    }
}

//! Репозиторий заявок/лидов (таблица `leads`).

use serde::Serialize;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

use crate::error::{AppError, AppResult};

/// Запись лида.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Lead {
    pub id: i64,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub message: String,
    pub source: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Входные данные для создания лида (id/статус/время проставляет хранилище).
#[derive(Debug, Clone, Serialize)]
pub struct NewLead {
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub message: String,
    pub source: String,
}

/// Сохраняет лид (status = "new") и возвращает полную запись с id.
pub async fn insert(pool: &SqlitePool, lead: NewLead) -> AppResult<Lead> {
    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        r#"INSERT INTO leads (name, email, phone, message, source, status, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, 'new', ?, ?)"#,
    )
    .bind(lead.name)
    .bind(lead.email)
    .bind(lead.phone)
    .bind(lead.message)
    .bind(lead.source)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    let id = result.last_insert_rowid();
    get(pool, id).await
}

/// Возвращает лид по id.
pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<Lead> {
    sqlx::query_as::<_, Lead>(
        "SELECT id, name, email, phone, message, source, status, created_at, updated_at FROM leads WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("lead #{id}")))
}

/// Возвращает последние `limit` лидов.
pub async fn list(pool: &SqlitePool, limit: i64) -> AppResult<Vec<Lead>> {
    sqlx::query_as::<_, Lead>(
        "SELECT id, name, email, phone, message, source, status, created_at, updated_at
         FROM leads ORDER BY created_at DESC, id DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

/// Обновляет статус лида.
pub async fn update_status(pool: &SqlitePool, id: i64, status: &str) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query("UPDATE leads SET status = ?, updated_at = ? WHERE id = ?")
        .bind(status)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("lead #{id}")));
    }
    Ok(())
}

/// Количество лидов за сегодня.
pub async fn count_today(pool: &SqlitePool) -> AppResult<i64> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM leads WHERE date(created_at) = date('now')")
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}

/// Распределение лидов по источникам (для donut-графика в админке).
pub async fn source_counts(pool: &SqlitePool) -> AppResult<Vec<(String, i64)>> {
    sqlx::query_as::<_, (String, i64)>(
        "SELECT source, COUNT(*) FROM leads GROUP BY source ORDER BY COUNT(*) DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::init_memory_pool;

    #[tokio::test]
    async fn insert_sets_new_status_and_returns_id() {
        let pool = init_memory_pool().await.unwrap();
        let lead = insert(
            &pool,
            NewLead {
                name: "Иван".into(),
                email: Some("i@example.com".into()),
                phone: Some("+7999".into()),
                message: "Нужен SEO".into(),
                source: "form".into(),
            },
        )
        .await
        .unwrap();

        assert!(lead.id > 0);
        assert_eq!(lead.status, "new");
        assert_eq!(lead.source, "form");
    }

    #[tokio::test]
    async fn update_status_changes_lead() {
        let pool = init_memory_pool().await.unwrap();
        let lead = insert(
            &pool,
            NewLead {
                name: "Иван".into(),
                email: None,
                phone: None,
                message: "msg".into(),
                source: "chat".into(),
            },
        )
        .await
        .unwrap();

        update_status(&pool, lead.id, "converted").await.unwrap();
        let updated = get(&pool, lead.id).await.unwrap();
        assert_eq!(updated.status, "converted");
    }

    #[tokio::test]
    async fn get_missing_returns_not_found() {
        let pool = init_memory_pool().await.unwrap();
        assert!(matches!(get(&pool, 999).await, Err(AppError::NotFound(_))));
    }
}

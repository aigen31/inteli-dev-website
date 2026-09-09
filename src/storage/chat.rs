//! Репозиторий чатов (таблица `chats`).

use serde::Serialize;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

use crate::error::AppResult;

/// Запись истории чата. Метки времени — RFC3339 строки (контролируем формат сами).
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ChatEntry {
    pub id: i64,
    pub user_message: String,
    pub bot_response: String,
    pub question_type: Option<String>,
    pub session_id: Option<String>,
    pub client_hash: Option<String>,
    pub created_at: String,
    pub response_time_ms: Option<i64>,
}

/// Входные данные для сохранения чата.
#[derive(Debug, Clone)]
pub struct NewChat {
    pub user_message: String,
    pub bot_response: String,
    pub question_type: Option<String>,
    pub session_id: Option<String>,
    pub client_hash: Option<String>,
    pub response_time_ms: Option<i64>,
}

/// Сохраняет чат и возвращает его id.
pub async fn insert(pool: &SqlitePool, chat: NewChat) -> AppResult<i64> {
    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        r#"INSERT INTO chats
           (user_message, bot_response, question_type, session_id, client_hash, created_at, response_time_ms)
           VALUES (?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(chat.user_message)
    .bind(chat.bot_response)
    .bind(chat.question_type)
    .bind(chat.session_id)
    .bind(chat.client_hash)
    .bind(now)
    .bind(chat.response_time_ms)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

/// Возвращает последние `limit` чатов (по убыванию времени).
pub async fn recent(pool: &SqlitePool, limit: i64) -> AppResult<Vec<ChatEntry>> {
    let rows = sqlx::query_as::<_, ChatEntry>(
        r#"SELECT id, user_message, bot_response, question_type, session_id, client_hash, created_at, response_time_ms
           FROM chats ORDER BY created_at DESC, id DESC LIMIT ?"#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Количество чатов за сегодня (UTC-дата).
pub async fn count_today(pool: &SqlitePool) -> AppResult<i64> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM chats WHERE date(created_at) = date('now')")
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}

/// Среднее время ответа (мс) по всем чатам с заполненным `response_time_ms`.
pub async fn avg_response_time_ms(pool: &SqlitePool) -> AppResult<Option<f64>> {
    let row: (Option<f64>,) = sqlx::query_as("SELECT AVG(response_time_ms) FROM chats")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

/// Распределение по типам вопросов (для аналитики preset-кнопок).
pub async fn question_type_counts(pool: &SqlitePool) -> AppResult<Vec<(String, i64)>> {
    let rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT question_type, COUNT(*) FROM chats WHERE question_type IS NOT NULL GROUP BY question_type ORDER BY COUNT(*) DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::init_memory_pool;

    #[tokio::test]
    async fn insert_and_recent_roundtrip() {
        let pool = init_memory_pool().await.unwrap();
        let id = insert(
            &pool,
            NewChat {
                user_message: "Сколько стоит?".into(),
                bot_response: "От 15 000 ₽".into(),
                question_type: Some("preset".into()),
                session_id: Some("sess-1".into()),
                client_hash: Some("abc".into()),
                response_time_ms: Some(123),
            },
        )
        .await
        .unwrap();
        assert!(id > 0);

        let rows = recent(&pool, 10).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].user_message, "Сколько стоит?");
        assert_eq!(rows[0].response_time_ms, Some(123));
    }

    #[tokio::test]
    async fn count_today_is_zero_initially() {
        let pool = init_memory_pool().await.unwrap();
        assert_eq!(count_today(&pool).await.unwrap(), 0);
    }
}

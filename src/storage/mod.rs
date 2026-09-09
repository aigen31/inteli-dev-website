//! Storage Layer — SQLite (WAL mode) для структурированных данных.
//!
//! Хранит историю чатов и заявки/лиды. Семантические знания об авторе живут в
//! OpenViking (`memory`), а не здесь — см. `memory-storage.md`.

pub mod chat;
pub mod lead;

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

use crate::error::AppResult;

/// Схема БД. Идемпотентна (CREATE TABLE IF NOT EXISTS).
pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS chats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_message TEXT NOT NULL,
    bot_response TEXT NOT NULL,
    question_type TEXT CHECK(question_type IN ('preset', 'free', 'analysis', 'lead_request')),
    session_id TEXT,
    client_hash TEXT,
    created_at TEXT NOT NULL,
    response_time_ms INTEGER
);

CREATE TABLE IF NOT EXISTS leads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT CHECK(length(email) <= 254),
    phone TEXT,
    message TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'form' CHECK(source IN ('form', 'chat', 'telegram', 'vk')),
    status TEXT NOT NULL DEFAULT 'new' CHECK(status IN ('new', 'processing', 'contacted', 'converted', 'dismissed')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS notification_retry_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    lead_id INTEGER NOT NULL REFERENCES leads(id),
    telegram_failed INTEGER DEFAULT 0,
    vk_failed INTEGER DEFAULT 0,
    attempt_count INTEGER DEFAULT 0,
    next_retry_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_error TEXT
);

CREATE INDEX IF NOT EXISTS idx_chats_created ON chats(created_at);
CREATE INDEX IF NOT EXISTS idx_chats_question_type ON chats(question_type);
CREATE INDEX IF NOT EXISTS idx_leads_status ON leads(status);
CREATE INDEX IF NOT EXISTS idx_leads_source ON leads(source);
"#;

/// Инициализирует пул соединений и применяет схему + PRAGMA-настройки.
pub async fn init_pool(path: &str) -> AppResult<SqlitePool> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("sqlite:{path}?mode=rwc"))
        .await?;

    // WAL для конкурентных чтений при записи (см. memory-storage.md).
    sqlx::raw_sql("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA cache_size=-64000;")
        .execute(&pool)
        .await?;

    sqlx::raw_sql(SCHEMA).execute(&pool).await?;

    Ok(pool)
}

/// Инициализирует in-memory пул для тестов.
#[cfg(test)]
pub async fn init_memory_pool() -> AppResult<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite::memory:")
        .await?;
    sqlx::raw_sql(SCHEMA).execute(&pool).await?;
    Ok(pool)
}

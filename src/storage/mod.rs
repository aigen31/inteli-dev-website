//! Storage Layer — SQLite (WAL mode) для структурированных данных.
//!
//! Хранит историю чатов и заявки/лиды. Семантические знания об авторе живут в
//! OpenViking (`memory`), а не здесь — см. `memory-storage.md`.

pub mod article;
pub mod chat;
pub mod lead;
pub mod settings;

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

use crate::error::AppResult;

/// Схема БД. Идемпотентна (CREATE TABLE IF NOT EXISTS).
pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS chats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_message TEXT NOT NULL,
    bot_response TEXT NOT NULL,
    question_type TEXT CHECK(question_type IN ('preset', 'free', 'analysis', 'lead_request', 'availability')),
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

CREATE TABLE IF NOT EXISTS site_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Статьи блога. Контент пишет владелец из админки (source = 'admin') либо,
-- в будущем, внешний пайплайн через API (source = 'n8n').
CREATE TABLE IF NOT EXISTS articles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    summary TEXT NOT NULL DEFAULT '',
    body_markdown TEXT NOT NULL,
    cover_image_url TEXT,
    -- Теги хранятся JSON-массивом строк: отдельная таблица тут не нужна,
    -- а фильтровать по ним в SQL мы не собираемся.
    tags TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft', 'published', 'archived')),
    published_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    -- Готовность к кросспостингу: источник и внешний ключ идемпотентности.
    source TEXT NOT NULL DEFAULT 'admin' CHECK(source IN ('admin', 'n8n', 'import')),
    external_id TEXT,
    canonical_url TEXT
);

-- Один и тот же внешний материал нельзя заимпортировать дважды: повторный
-- прогон n8n с тем же external_id обновит статью, а не создаст дубль.
CREATE UNIQUE INDEX IF NOT EXISTS idx_articles_external
    ON articles(source, external_id) WHERE external_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_articles_public ON articles(status, published_at);
CREATE INDEX IF NOT EXISTS idx_articles_updated ON articles(updated_at);

-- Transactional outbox для кросспостинга (n8n): событие пишется в той же
-- транзакции, что и изменение статьи, поэтому «опубликовано, но событие
-- потерялось» невозможно. Читатель (n8n) забирает pending-события и
-- подтверждает их через ack.
--
-- article_id НЕ объявлен как FOREIGN KEY сознательно: это неизменяемый лог
-- доставки, он должен переживать удаление статьи — событие article.deleted
-- обязано дойти до читателя, даже когда строки статьи уже нет.
CREATE TABLE IF NOT EXISTS article_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    article_id INTEGER NOT NULL,
    article_slug TEXT NOT NULL,
    event_type TEXT NOT NULL
        CHECK(event_type IN ('article.published', 'article.updated', 'article.unpublished', 'article.deleted')),
    payload TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'delivered', 'failed')),
    attempts INTEGER NOT NULL DEFAULT 0,
    -- Задел под push-доставку с backoff: pull-читателю не нужен, но колонка
    -- есть, чтобы включить ретраи без миграции.
    next_attempt_at TEXT,
    created_at TEXT NOT NULL,
    delivered_at TEXT,
    last_error TEXT
);
CREATE INDEX IF NOT EXISTS idx_article_events_status ON article_events(status, id);

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

    // CREATE TABLE IF NOT EXISTS не меняет уже созданные таблицы, поэтому
    // расширение CHECK-ограничения требует отдельной миграции.
    migrate_chats_question_type(&pool).await?;

    Ok(pool)
}

/// Расширяет CHECK-ограничение `chats.question_type` значением `availability`.
///
/// До этой миграции вопросы «когда вы свободны?» не сохранялись: вставка
/// нарушала CHECK, а `ChatService::save_chat` глотает ошибку записи, чтобы сбой
/// истории не ломал ответ пользователю. В итоге вопрос терялся молча.
///
/// SQLite не умеет менять CHECK через ALTER, поэтому таблица пересобирается:
/// новая схема → копия данных → подмена. Всё в одной транзакции, поэтому
/// обрыв на середине не оставит БД в полусобранном состоянии.
async fn migrate_chats_question_type(pool: &SqlitePool) -> AppResult<()> {
    let table_sql: Option<String> =
        sqlx::query_scalar("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'chats'")
            .fetch_optional(pool)
            .await?;

    let Some(table_sql) = table_sql else {
        return Ok(());
    };
    if table_sql.contains("'availability'") {
        return Ok(());
    }

    tracing::info!("миграция chats: добавляем 'availability' в CHECK(question_type)");

    let mut tx = pool.begin().await?;
    sqlx::raw_sql(
        r#"
CREATE TABLE chats_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_message TEXT NOT NULL,
    bot_response TEXT NOT NULL,
    question_type TEXT CHECK(question_type IN ('preset', 'free', 'analysis', 'lead_request', 'availability')),
    session_id TEXT,
    client_hash TEXT,
    created_at TEXT NOT NULL,
    response_time_ms INTEGER
);
INSERT INTO chats_new SELECT id, user_message, bot_response, question_type, session_id, client_hash, created_at, response_time_ms FROM chats;
DROP TABLE chats;
ALTER TABLE chats_new RENAME TO chats;
CREATE INDEX IF NOT EXISTS idx_chats_created ON chats(created_at);
CREATE INDEX IF NOT EXISTS idx_chats_question_type ON chats(question_type);
"#,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Схема `chats` до миграции: без `availability` в CHECK.
    const OLD_CHATS_SCHEMA: &str = r#"
CREATE TABLE chats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_message TEXT NOT NULL,
    bot_response TEXT NOT NULL,
    question_type TEXT CHECK(question_type IN ('preset', 'free', 'analysis', 'lead_request')),
    session_id TEXT,
    client_hash TEXT,
    created_at TEXT NOT NULL,
    response_time_ms INTEGER
);
"#;

    async fn temp_db() -> (SqlitePool, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!("inteli_mig_{}.db", uuid::Uuid::new_v4()));
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite:{}?mode=rwc", path.display()))
            .await
            .unwrap();
        (pool, path)
    }

    async fn insert_chat(pool: &SqlitePool, question_type: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO chats (user_message, bot_response, question_type, created_at) VALUES ('q', 'a', ?, '2026-09-24T00:00:00Z')",
        )
        .bind(question_type)
        .execute(pool)
        .await
        .map(|_| ())
    }

    #[tokio::test]
    async fn old_schema_rejects_availability_before_migration() {
        // Фиксируем сам баг: до миграции вставка падала на CHECK, а
        // save_chat глотает ошибку — вопрос «когда вы свободны?» терялся молча.
        let (pool, path) = temp_db().await;
        sqlx::raw_sql(OLD_CHATS_SCHEMA).execute(&pool).await.unwrap();
        assert!(insert_chat(&pool, "availability").await.is_err());

        pool.close().await;
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
    }

    #[tokio::test]
    async fn migration_keeps_rows_and_allows_availability() {
        let (pool, path) = temp_db().await;
        sqlx::raw_sql(OLD_CHATS_SCHEMA).execute(&pool).await.unwrap();
        insert_chat(&pool, "preset").await.unwrap();
        insert_chat(&pool, "free").await.unwrap();
        pool.close().await;

        // Повторный init_pool видит старую схему и пересобирает таблицу.
        let pool = init_pool(path.to_str().unwrap()).await.unwrap();

        // Данные уцелели.
        let kept: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chats")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(kept, 2);

        // И новые значения теперь проходят.
        insert_chat(&pool, "availability").await.unwrap();
        let stored: Vec<String> =
            sqlx::query_scalar("SELECT question_type FROM chats ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(stored, vec!["preset", "free", "availability"]);

        // Миграция идемпотентна: повторный запуск ничего не ломает.
        pool.close().await;
        let pool = init_pool(path.to_str().unwrap()).await.unwrap();
        let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chats")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(after, 3);

        pool.close().await;
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
    }

    #[tokio::test]
    async fn fresh_schema_is_not_migrated() {
        let (pool, path) = temp_db().await;
        pool.close().await;
        let pool = init_pool(path.to_str().unwrap()).await.unwrap();

        // На свежей БД миграция не нужна — сразу можно писать availability.
        insert_chat(&pool, "availability").await.unwrap();

        pool.close().await;
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
    }
}

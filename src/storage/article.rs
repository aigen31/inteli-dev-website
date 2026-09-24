//! Репозиторий статей блога и outbox-событий кросспостинга.
//!
//! Слой хранилища оперирует «сырыми» колонками: `status` и `tags` здесь
//! обычные строки, а доменные типы (`ArticleStatus`, `Vec<String>`) живут в
//! [`crate::services::article`]. Так SQL не зависит от модели предметной
//! области, а модель — от схемы таблицы.
//!
//! # Почему запись статьи и события — одна транзакция
//!
//! Outbox-паттерн имеет смысл, только если «статья опубликована» и «событие
//! для n8n создано» происходят атомарно. Если писать их двумя запросами, то
//! обрыв между ними даёт либо потерянный пост (опубликовали, n8n не узнал),
//! либо фантомное событие. Поэтому [`insert`], [`update`] и [`delete`] сами
//! открывают транзакцию и принимают payload события замыканием: оно получает
//! уже сохранённую строку, то есть видит настоящие `id` и таймстемпы.

use serde::Serialize;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

use crate::error::{AppError, AppResult};

/// Строка таблицы `articles` в том виде, в каком она лежит в БД.
#[derive(Debug, Clone, FromRow)]
pub struct ArticleRow {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub body_markdown: String,
    pub cover_image_url: Option<String>,
    /// JSON-массив строк.
    pub tags: String,
    pub status: String,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub source: String,
    pub external_id: Option<String>,
    pub canonical_url: Option<String>,
}

/// Данные для создания статьи. `id` и таймстемпы проставляет хранилище.
#[derive(Debug, Clone)]
pub struct NewArticle {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub body_markdown: String,
    pub cover_image_url: Option<String>,
    /// JSON-массив строк.
    pub tags: String,
    pub status: String,
    pub published_at: Option<String>,
    pub source: String,
    pub external_id: Option<String>,
    pub canonical_url: Option<String>,
}

/// Полная замена редактируемых полей статьи.
#[derive(Debug, Clone)]
pub struct ArticlePatch {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub body_markdown: String,
    pub cover_image_url: Option<String>,
    /// JSON-массив строк.
    pub tags: String,
    pub status: String,
    pub published_at: Option<String>,
    pub canonical_url: Option<String>,
}

/// Событие outbox в том виде, в каком оно лежит в БД.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ArticleEvent {
    pub id: i64,
    pub article_id: i64,
    pub article_slug: String,
    pub event_type: String,
    pub payload: String,
    pub status: String,
    pub attempts: i64,
    pub next_attempt_at: Option<String>,
    pub created_at: String,
    pub delivered_at: Option<String>,
    pub last_error: Option<String>,
}

/// Сколько раз событие может упасть, прежде чем читатель перестанет его
/// получать. Защита от «отравленного» события, которое роняет пайплайн n8n.
pub const MAX_EVENT_ATTEMPTS: i64 = 5;

/// Создаёт статью. Если передано событие, оно пишется в той же транзакции.
pub async fn insert<F>(
    pool: &SqlitePool,
    new: NewArticle,
    event: Option<(String, F)>,
) -> AppResult<ArticleRow>
where
    F: FnOnce(&ArticleRow) -> String,
{
    let now = chrono::Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;

    let result = sqlx::query(
        r#"INSERT INTO articles
           (slug, title, summary, body_markdown, cover_image_url, tags, status, published_at,
            created_at, updated_at, source, external_id, canonical_url)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&new.slug)
    .bind(&new.title)
    .bind(&new.summary)
    .bind(&new.body_markdown)
    .bind(&new.cover_image_url)
    .bind(&new.tags)
    .bind(&new.status)
    .bind(&new.published_at)
    .bind(&now)
    .bind(&now)
    .bind(&new.source)
    .bind(&new.external_id)
    .bind(&new.canonical_url)
    .execute(&mut *tx)
    .await
    .map_err(map_unique_violation)?;

    let id = result.last_insert_rowid();
    let row = fetch(&mut *tx, id).await?;

    if let Some((event_type, payload)) = event {
        insert_event_tx(&mut tx, &row, &event_type, &payload(&row)).await?;
    }

    tx.commit().await?;
    Ok(row)
}

/// Полностью заменяет редактируемые поля статьи.
pub async fn update<F>(
    pool: &SqlitePool,
    id: i64,
    patch: ArticlePatch,
    event: Option<(String, F)>,
) -> AppResult<ArticleRow>
where
    F: FnOnce(&ArticleRow) -> String,
{
    let now = chrono::Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;

    let result = sqlx::query(
        r#"UPDATE articles SET
             slug = ?, title = ?, summary = ?, body_markdown = ?, cover_image_url = ?,
             tags = ?, status = ?, published_at = ?, updated_at = ?, canonical_url = ?
           WHERE id = ?"#,
    )
    .bind(&patch.slug)
    .bind(&patch.title)
    .bind(&patch.summary)
    .bind(&patch.body_markdown)
    .bind(&patch.cover_image_url)
    .bind(&patch.tags)
    .bind(&patch.status)
    .bind(&patch.published_at)
    .bind(&now)
    .bind(&patch.canonical_url)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(map_unique_violation)?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("article #{id}")));
    }

    let row = fetch(&mut *tx, id).await?;

    if let Some((event_type, payload)) = event {
        insert_event_tx(&mut tx, &row, &event_type, &payload(&row)).await?;
    }

    tx.commit().await?;
    Ok(row)
}

/// Удаляет статью. Замыкание события получает строку **до** удаления — иначе
/// событию `article.deleted` нечего было бы положить в payload.
pub async fn delete<F>(
    pool: &SqlitePool,
    id: i64,
    event: Option<(String, F)>,
) -> AppResult<Option<ArticleRow>>
where
    F: FnOnce(&ArticleRow) -> String,
{
    let mut tx = pool.begin().await?;

    let row = sqlx::query_as::<_, ArticleRow>(
        "SELECT id, slug, title, summary, body_markdown, cover_image_url, tags, status,
                published_at, created_at, updated_at, source, external_id, canonical_url
         FROM articles WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    if let Some((event_type, payload)) = event {
        insert_event_tx(&mut tx, &row, &event_type, &payload(&row)).await?;
    }

    // События переживают статью (см. комментарий к схеме), поэтому отдельной
    // чистки outbox здесь нет: это неизменяемый лог доставки.
    sqlx::query("DELETE FROM articles WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Some(row))
}

/// Читает строку внутри уже открытой транзакции.
async fn fetch<'e, E>(executor: E, id: i64) -> AppResult<ArticleRow>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    sqlx::query_as::<_, ArticleRow>(
        "SELECT id, slug, title, summary, body_markdown, cover_image_url, tags, status,
                published_at, created_at, updated_at, source, external_id, canonical_url
         FROM articles WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(executor)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("article #{id}")))
}

/// Возвращает статью по id.
pub async fn get(pool: &SqlitePool, id: i64) -> AppResult<ArticleRow> {
    fetch(pool, id).await
}

/// Возвращает статью по id или `None`.
pub async fn find(pool: &SqlitePool, id: i64) -> AppResult<Option<ArticleRow>> {
    sqlx::query_as::<_, ArticleRow>(
        "SELECT id, slug, title, summary, body_markdown, cover_image_url, tags, status,
                published_at, created_at, updated_at, source, external_id, canonical_url
         FROM articles WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

/// Возвращает статью по slug (в том числе черновик — фильтрует вызывающий).
pub async fn find_by_slug(pool: &SqlitePool, slug: &str) -> AppResult<Option<ArticleRow>> {
    sqlx::query_as::<_, ArticleRow>(
        "SELECT id, slug, title, summary, body_markdown, cover_image_url, tags, status,
                published_at, created_at, updated_at, source, external_id, canonical_url
         FROM articles WHERE slug = ?",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

/// Ищет статью по внешнему ключу идемпотентности (повторный импорт от n8n).
pub async fn find_by_external(
    pool: &SqlitePool,
    source: &str,
    external_id: &str,
) -> AppResult<Option<ArticleRow>> {
    sqlx::query_as::<_, ArticleRow>(
        "SELECT id, slug, title, summary, body_markdown, cover_image_url, tags, status,
                published_at, created_at, updated_at, source, external_id, canonical_url
         FROM articles WHERE source = ? AND external_id = ?",
    )
    .bind(source)
    .bind(external_id)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

/// Список статей для админки: любые статусы, свежие сверху.
///
/// `status = None` — без фильтра. `limit`/`offset` — пагинация.
pub async fn list(
    pool: &SqlitePool,
    status: Option<&str>,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<ArticleRow>> {
    let sql = match status {
        Some(_) => {
            "SELECT id, slug, title, summary, body_markdown, cover_image_url, tags, status,
                     published_at, created_at, updated_at, source, external_id, canonical_url
              FROM articles WHERE status = ?
              ORDER BY updated_at DESC, id DESC LIMIT ? OFFSET ?"
        }
        None => {
            "SELECT id, slug, title, summary, body_markdown, cover_image_url, tags, status,
                     published_at, created_at, updated_at, source, external_id, canonical_url
              FROM articles ORDER BY updated_at DESC, id DESC LIMIT ? OFFSET ?"
        }
    };

    let mut query = sqlx::query_as::<_, ArticleRow>(sql);
    if let Some(status) = status {
        query = query.bind(status);
    }
    query
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
}

/// Все опубликованные статьи вместе с телом.
///
/// Читается целиком при старте и после каждой мутации: Leptos рендерит
/// SSR-страницы синхронно, поэтому список блога и страница статьи должны
/// обслуживаться из памяти процесса, а не из БД. Для персонального блога
/// (десятки статей) это килобайты, а не мегабайты.
pub async fn list_published_full(pool: &SqlitePool) -> AppResult<Vec<ArticleRow>> {
    sqlx::query_as::<_, ArticleRow>(
        "SELECT id, slug, title, summary, body_markdown, cover_image_url, tags, status,
                published_at, created_at, updated_at, source, external_id, canonical_url
         FROM articles WHERE status = 'published' AND published_at IS NOT NULL
         ORDER BY published_at DESC, id DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

/// Считает статьи (для пагинации и KPI).
pub async fn count(pool: &SqlitePool, status: Option<&str>) -> AppResult<i64> {
    let row: (i64,) = match status {
        Some(status) => {
            sqlx::query_as("SELECT COUNT(*) FROM articles WHERE status = ?")
                .bind(status)
                .fetch_one(pool)
                .await?
        }
        None => {
            sqlx::query_as("SELECT COUNT(*) FROM articles")
                .fetch_one(pool)
                .await?
        }
    };
    Ok(row.0)
}

/// Занят ли slug (с учётом исключения при обновлении той же статьи).
pub async fn slug_taken(pool: &SqlitePool, slug: &str, exclude_id: Option<i64>) -> AppResult<bool> {
    let row: (i64,) = match exclude_id {
        Some(id) => {
            sqlx::query_as("SELECT COUNT(*) FROM articles WHERE slug = ? AND id != ?")
                .bind(slug)
                .bind(id)
                .fetch_one(pool)
                .await?
        }
        None => {
            sqlx::query_as("SELECT COUNT(*) FROM articles WHERE slug = ?")
                .bind(slug)
                .fetch_one(pool)
                .await?
        }
    };
    Ok(row.0 > 0)
}

// ---------------------------------------------------------------------------
// Outbox
// ---------------------------------------------------------------------------

/// Пишет событие в outbox внутри переданной транзакции.
async fn insert_event_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    row: &ArticleRow,
    event_type: &str,
    payload: &str,
) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        r#"INSERT INTO article_events
           (article_id, article_slug, event_type, payload, status, attempts, created_at)
           VALUES (?, ?, ?, ?, 'pending', 0, ?)"#,
    )
    .bind(row.id)
    .bind(&row.slug)
    .bind(event_type)
    .bind(payload)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Список событий для читателя-пайплайна.
pub async fn list_events(
    pool: &SqlitePool,
    status: &str,
    limit: i64,
) -> AppResult<Vec<ArticleEvent>> {
    sqlx::query_as::<_, ArticleEvent>(
        "SELECT id, article_id, article_slug, event_type, payload, status, attempts,
                next_attempt_at, created_at, delivered_at, last_error
         FROM article_events WHERE status = ? ORDER BY id ASC LIMIT ?",
    )
    .bind(status)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

/// Помечает события доставленными. Возвращает число реально изменённых строк.
///
/// Повторный ack идемпотентен: уже доставленные события не считаются, поэтому
/// n8n может подтверждать один и тот же батч повторно после сетевого сбоя.
pub async fn ack_events(pool: &SqlitePool, ids: &[i64]) -> AppResult<u64> {
    if ids.is_empty() {
        return Ok(0);
    }

    let now = chrono::Utc::now().to_rfc3339();
    let mut changed = 0u64;
    let mut tx = pool.begin().await?;
    for id in ids {
        let result = sqlx::query(
            "UPDATE article_events SET status = 'delivered', delivered_at = ?, last_error = NULL
             WHERE id = ? AND status = 'pending'",
        )
        .bind(&now)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        changed += result.rows_affected();
    }
    tx.commit().await?;

    Ok(changed)
}

/// Регистрирует неудачную доставку: инкремент попыток и перевод в `failed`
/// после [`MAX_EVENT_ATTEMPTS`].
pub async fn fail_event(pool: &SqlitePool, id: i64, error: &str) -> AppResult<()> {
    sqlx::query(
        r#"UPDATE article_events
           SET attempts = attempts + 1,
               last_error = ?,
               status = CASE WHEN attempts + 1 >= ? THEN 'failed' ELSE 'pending' END
           WHERE id = ?"#,
    )
    .bind(error)
    .bind(MAX_EVENT_ATTEMPTS)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Сколько событий ждёт доставки (для бейджа в админке).
pub async fn pending_event_count(pool: &SqlitePool) -> AppResult<i64> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM article_events WHERE status = 'pending'")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

/// Человекочитаемое сообщение о нарушении UNIQUE-индекса вместо «UNIQUE
/// constraint failed: index ...».
fn map_unique_violation(e: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db) = &e {
        if db.message().contains("UNIQUE") {
            return AppError::Validation("статья с таким адресом (slug) уже существует".into());
        }
    }
    AppError::Database(e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::init_memory_pool;

    fn new_article(slug: &str, status: &str) -> NewArticle {
        NewArticle {
            slug: slug.into(),
            title: "Заголовок".into(),
            summary: "Кратко".into(),
            body_markdown: "# Текст".into(),
            cover_image_url: None,
            tags: r#"["rust"]"#.into(),
            status: status.into(),
            published_at: (status == "published").then(|| "2026-09-24T10:00:00Z".to_string()),
            source: "admin".into(),
            external_id: None,
            canonical_url: None,
        }
    }

    fn patch(status: &str) -> ArticlePatch {
        ArticlePatch {
            slug: "slug".into(),
            title: "Новый".into(),
            summary: "s".into(),
            body_markdown: "b".into(),
            cover_image_url: None,
            tags: "[]".into(),
            status: status.into(),
            published_at: None,
            canonical_url: None,
        }
    }

    #[tokio::test]
    async fn insert_and_get_roundtrip() {
        let pool = init_memory_pool().await.unwrap();
        let row = insert(&pool, new_article("hello", "draft"), None::<(String, fn(&ArticleRow) -> String)>)
            .await
            .unwrap();

        assert!(row.id > 0);
        assert_eq!(row.slug, "hello");
        assert_eq!(row.status, "draft");
        assert_eq!(row.tags, r#"["rust"]"#);
    }

    #[tokio::test]
    async fn duplicate_slug_is_a_validation_error() {
        let pool = init_memory_pool().await.unwrap();
        insert(&pool, new_article("dup", "draft"), None::<(String, fn(&ArticleRow) -> String)>)
            .await
            .unwrap();

        let err = insert(&pool, new_article("dup", "draft"), None::<(String, fn(&ArticleRow) -> String)>)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Validation(_)), "получили {err:?}");
    }

    #[tokio::test]
    async fn insert_with_event_is_atomic() {
        let pool = init_memory_pool().await.unwrap();
        let row = insert(
            &pool,
            new_article("with-event", "published"),
            Some(("article.published".to_string(), |r: &ArticleRow| {
                format!(r#"{{"id":{},"slug":"{}"}}"#, r.id, r.slug)
            })),
        )
        .await
        .unwrap();

        let events = list_events(&pool, "pending", 10).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].article_id, row.id);
        assert_eq!(events[0].event_type, "article.published");
        // Замыкание видит уже сохранённую строку — в payload настоящий id.
        assert!(events[0].payload.contains(&format!(r#""id":{}"#, row.id)));
    }

    #[tokio::test]
    async fn update_replaces_fields_and_emits_event() {
        let pool = init_memory_pool().await.unwrap();
        let row = insert(&pool, new_article("upd", "draft"), None::<(String, fn(&ArticleRow) -> String)>)
            .await
            .unwrap();

        let updated = update(
            &pool,
            row.id,
            patch("published"),
            Some(("article.published".to_string(), |_: &ArticleRow| "{}".to_string())),
        )
        .await
        .unwrap();

        assert_eq!(updated.title, "Новый");
        assert_eq!(updated.status, "published");
        assert_eq!(list_events(&pool, "pending", 10).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn delete_returns_row_and_keeps_event() {
        let pool = init_memory_pool().await.unwrap();
        let row = insert(&pool, new_article("gone", "published"), None::<(String, fn(&ArticleRow) -> String)>)
            .await
            .unwrap();

        let deleted = delete(
            &pool,
            row.id,
            Some(("article.deleted".to_string(), |r: &ArticleRow| {
                format!(r#"{{"slug":"{}"}}"#, r.slug)
            })),
        )
        .await
        .unwrap()
        .expect("статья должна была существовать");

        assert_eq!(deleted.slug, "gone");
        assert!(find(&pool, row.id).await.unwrap().is_none());

        // Событие об удалении обязано пережить саму статью.
        let events = list_events(&pool, "pending", 10).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "article.deleted");
        assert!(events[0].payload.contains(r#""slug":"gone""#));
    }

    #[tokio::test]
    async fn delete_missing_returns_none() {
        let pool = init_memory_pool().await.unwrap();
        assert!(delete(&pool, 404, None::<(String, fn(&ArticleRow) -> String)>)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn list_filters_by_status_and_paginates() {
        let pool = init_memory_pool().await.unwrap();
        for i in 0..3 {
            insert(
                &pool,
                new_article(&format!("pub-{i}"), "published"),
                None::<(String, fn(&ArticleRow) -> String)>,
            )
            .await
            .unwrap();
        }
        insert(&pool, new_article("draft-1", "draft"), None::<(String, fn(&ArticleRow) -> String)>)
            .await
            .unwrap();

        assert_eq!(list(&pool, Some("published"), 10, 0).await.unwrap().len(), 3);
        assert_eq!(list(&pool, Some("draft"), 10, 0).await.unwrap().len(), 1);
        assert_eq!(list(&pool, None, 10, 0).await.unwrap().len(), 4);
        assert_eq!(list(&pool, None, 2, 0).await.unwrap().len(), 2);
        assert_eq!(list(&pool, None, 2, 2).await.unwrap().len(), 2);
        assert_eq!(count(&pool, Some("published")).await.unwrap(), 3);
    }

    #[tokio::test]
    async fn published_full_loads_bodies_and_skips_drafts() {
        let pool = init_memory_pool().await.unwrap();
        insert(&pool, new_article("live", "published"), None::<(String, fn(&ArticleRow) -> String)>)
            .await
            .unwrap();
        insert(&pool, new_article("hidden", "draft"), None::<(String, fn(&ArticleRow) -> String)>)
            .await
            .unwrap();

        let rows = list_published_full(&pool).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].slug, "live");
        // Тело нужно целиком: из него рендерится страница статьи и считается
        // время чтения в списке блога.
        assert_eq!(rows[0].body_markdown, "# Текст");
    }

    #[tokio::test]
    async fn slug_taken_ignores_own_row() {
        let pool = init_memory_pool().await.unwrap();
        let row = insert(&pool, new_article("self", "draft"), None::<(String, fn(&ArticleRow) -> String)>)
            .await
            .unwrap();

        assert!(slug_taken(&pool, "self", None).await.unwrap());
        assert!(!slug_taken(&pool, "self", Some(row.id)).await.unwrap());
    }

    #[tokio::test]
    async fn external_id_is_unique_per_source() {
        let pool = init_memory_pool().await.unwrap();
        let mut first = new_article("ext-1", "draft");
        first.source = "n8n".into();
        first.external_id = Some("tg-42".into());
        insert(&pool, first, None::<(String, fn(&ArticleRow) -> String)>)
            .await
            .unwrap();

        let found = find_by_external(&pool, "n8n", "tg-42").await.unwrap();
        assert!(found.is_some());

        // Повторный импорт того же материала — не дубль, а ошибка уникальности.
        let mut dupe = new_article("ext-2", "draft");
        dupe.source = "n8n".into();
        dupe.external_id = Some("tg-42".into());
        assert!(insert(&pool, dupe, None::<(String, fn(&ArticleRow) -> String)>)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn ack_is_idempotent_and_fail_gives_up_after_max_attempts() {
        let pool = init_memory_pool().await.unwrap();
        insert(
            &pool,
            new_article("ack", "published"),
            Some(("article.published".to_string(), |_: &ArticleRow| "{}".to_string())),
        )
        .await
        .unwrap();
        let event_id = list_events(&pool, "pending", 10).await.unwrap()[0].id;

        assert_eq!(ack_events(&pool, &[event_id]).await.unwrap(), 1);
        // Повторный ack ничего не меняет — n8n может переподтвердить батч.
        assert_eq!(ack_events(&pool, &[event_id]).await.unwrap(), 0);
        assert_eq!(pending_event_count(&pool).await.unwrap(), 0);
        assert!(list_events(&pool, "delivered", 10).await.unwrap()[0]
            .delivered_at
            .is_some());

        // А вот падения копятся и в итоге переводят событие в failed.
        insert(
            &pool,
            new_article("ack-2", "published"),
            Some(("article.published".to_string(), |_: &ArticleRow| "{}".to_string())),
        )
        .await
        .unwrap();
        let failing = list_events(&pool, "pending", 10).await.unwrap()[0].id;

        for _ in 0..MAX_EVENT_ATTEMPTS {
            fail_event(&pool, failing, "502 от n8n").await.unwrap();
        }
        assert_eq!(list_events(&pool, "pending", 10).await.unwrap().len(), 0);
        let failed = list_events(&pool, "failed", 10).await.unwrap();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].last_error.as_deref(), Some("502 от n8n"));
    }

    #[tokio::test]
    async fn ack_empty_list_is_noop() {
        let pool = init_memory_pool().await.unwrap();
        assert_eq!(ack_events(&pool, &[]).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn missing_article_is_not_found() {
        let pool = init_memory_pool().await.unwrap();
        assert!(matches!(get(&pool, 7).await, Err(AppError::NotFound(_))));
        assert!(find_by_slug(&pool, "nope").await.unwrap().is_none());
    }
}

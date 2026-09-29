//! Репозиторий изображений статей.
//!
//! # Почему картинки лежат в SQLite, а не на диске
//!
//! Файловая система потребовала бы второго volume, отдельной синхронизации
//! бэкапов и отдельного набора правил по путям (обход каталога, права, гонки
//! при записи). В SQLite картинка — это строки в той же БД, что и статьи: один
//! volume, один бэкап, одна транзакция, а имя файла в URL жёстко проверяется по
//! белому списку, поэтому «выйти из каталога» нечем.
//!
//! Дедупликация — по `sha256`: один и тот же файл, загруженный дважды, занимает
//! одну строку, а в тексте статьи появляется один и тот же адрес.

use serde::Serialize;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

use crate::error::{AppError, AppResult};

/// Строка таблицы `media` без тела файла: нужно для отдачи и для метаданных.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct MediaRow {
    pub id: i64,
    pub name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub sha256: String,
    pub created_at: String,
}

/// Строка вместе с содержимым файла.
#[derive(Debug, Clone, FromRow)]
pub struct MediaBlob {
    pub name: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
    pub size_bytes: i64,
    pub sha256: String,
}

/// Сохраняет файл и возвращает строку вместе с признаком «файл новый».
///
/// `deduplicated = false` означает, что точно такой же файл уже был в базе: в
/// этом случае возвращается **существующая** строка, а новая не создаётся.
/// Поэтому повторная загрузка не плодит копии и не меняет адрес картинки в уже
/// опубликованных статьях.
pub async fn insert(
    pool: &SqlitePool,
    name: &str,
    content_type: &str,
    bytes: &[u8],
    sha256: &str,
) -> AppResult<(MediaRow, bool)> {
    let now = chrono::Utc::now().to_rfc3339();
    let size_bytes = bytes.len() as i64;

    let mut tx = pool.begin().await?;
    let result = sqlx::query(
        r#"INSERT OR IGNORE INTO media (name, content_type, bytes, size_bytes, sha256, created_at)
           VALUES (?, ?, ?, ?, ?, ?)"#,
    )
    .bind(name)
    .bind(content_type)
    .bind(bytes)
    .bind(size_bytes)
    .bind(sha256)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 1 {
        let row = MediaRow {
            id: result.last_insert_rowid(),
            name: name.to_string(),
            content_type: content_type.to_string(),
            size_bytes,
            sha256: sha256.to_string(),
            created_at: now,
        };
        tx.commit().await?;
        return Ok((row, true));
    }

    // `INSERT OR IGNORE` мог сработать по двум причинам: такой файл уже есть
    // (обычный случай) или имя занято другим содержимым. Различаем их по хешу:
    // имя — производное от хеша, поэтому второй вариант практически невозможен,
    // но молча вернуть чужую картинку в ответе на загрузку было бы хуже ошибки.
    let existing = sqlx::query_as::<_, MediaRow>(
        "SELECT id, name, content_type, size_bytes, sha256, created_at
         FROM media WHERE sha256 = ?",
    )
    .bind(sha256)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        AppError::Internal(format!(
            "media: имя `{name}` занято другим файлом (хеш {sha256})"
        ))
    })?;

    tx.commit().await?;
    Ok((existing, false))
}

/// Ищет файл по имени (то, что стоит в URL) вместе с содержимым.
pub async fn find_blob(pool: &SqlitePool, name: &str) -> AppResult<Option<MediaBlob>> {
    sqlx::query_as::<_, MediaBlob>(
        "SELECT name, content_type, bytes, size_bytes, sha256 FROM media WHERE name = ?",
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

/// Сколько файлов и сколько байт занимает медиатека.
///
/// Показывается в админке: загруженные, но так и не использованные картинки
/// (черновик закрыли, не сохранив) копятся молча, и владельцу нужен хотя бы
/// видимый счётчик.
pub async fn stats(pool: &SqlitePool) -> AppResult<(i64, i64)> {
    let row: (i64, i64) =
        sqlx::query_as("SELECT COUNT(*), COALESCE(SUM(size_bytes), 0) FROM media")
            .fetch_one(pool)
            .await?;
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::init_memory_pool;

    #[tokio::test]
    async fn insert_and_find_blob_roundtrip() {
        let pool = init_memory_pool().await.unwrap();
        let bytes = b"\x89PNG\r\n\x1a\n-dummy".to_vec();

        let (row, created) = insert(&pool, "abc.png", "image/png", &bytes, "hash-1")
            .await
            .unwrap();
        assert!(created);
        assert_eq!(row.name, "abc.png");
        assert_eq!(row.size_bytes, bytes.len() as i64);

        let blob = find_blob(&pool, "abc.png").await.unwrap().expect("файл есть");
        assert_eq!(blob.bytes, bytes);
        assert_eq!(blob.content_type, "image/png");

        assert!(find_blob(&pool, "missing.png").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn identical_bytes_are_deduplicated() {
        let pool = init_memory_pool().await.unwrap();
        let bytes = b"same-content".to_vec();

        let (first, created) = insert(&pool, "aaa.png", "image/png", &bytes, "hash-same")
            .await
            .unwrap();
        assert!(created);

        // Второй раз тот же файл: новая строка не создаётся, возвращается
        // существующая — адрес картинки в статье не меняется.
        let (second, created_again) = insert(&pool, "bbb.png", "image/png", &bytes, "hash-same")
            .await
            .unwrap();
        assert!(!created_again);
        assert_eq!(second.id, first.id);
        assert_eq!(second.name, "aaa.png");

        let (count, total) = stats(&pool).await.unwrap();
        assert_eq!(count, 1);
        assert_eq!(total, bytes.len() as i64);
    }

    #[tokio::test]
    async fn name_collision_with_another_hash_is_an_error() {
        let pool = init_memory_pool().await.unwrap();
        insert(&pool, "dup.png", "image/png", b"one", "hash-a")
            .await
            .unwrap();

        // Тот же адрес, другое содержимое: отдать чужую картинку нельзя.
        let err = insert(&pool, "dup.png", "image/png", b"two", "hash-b")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Internal(_)), "получили {err:?}");
    }
}

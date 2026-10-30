//! Прайс-лист: строки, которые владелец правит в админке.
//!
//! Почему отдельная таблица, а не поля услуг в контенте: услуги приходят из
//! OpenViking и описывают направление («приватные AI-системы»), а прайс — это
//! то, что продаётся и меняется чаще: пакеты, часы поддержки, аудит, обучение.
//! Один и тот же пункт может не совпадать с услугой.
//!
//! Источник правды для ассистента — эта таблица. Если она пуста (первый
//! запуск), она заполняется диапазонами из контента, чтобы ИИ не отвечал
//! «цен нет» на вопрос о цене.

use sqlx::sqlite::SqlitePool;

use crate::error::AppResult;
use crate::storage::price_catalog;

/// Строка прайса.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PriceItem {
    /// Стабильный идентификатор строки (используется в правках из админки).
    pub slug: String,
    pub title: String,
    pub description: String,
    /// Цена «от», строка: в прайсе встречается и «по запросу».
    pub price_from: String,
    /// Цена «до», пустая — открытая вилка.
    pub price_to: String,
    /// Единица: проект, месяц, час, аудит.
    pub unit: String,
    /// Порядок показа.
    pub sort: i64,
    /// Скрытые строки не попадают ни в прайс на сайте, ни в ответы ИИ.
    pub is_active: bool,
    pub updated_at: String,
}

impl PriceItem {
    /// Одна строка для промпта и MCP: «Название — от X до Y ₽, единица».
    pub fn line(&self) -> String {
        let range = match (self.price_from.trim(), self.price_to.trim()) {
            ("", "") => "по запросу".to_string(),
            (from, "") => format!("от {from}"),
            ("", to) => format!("до {to}"),
            (from, to) => format!("{from} – {to}"),
        };

        let unit = if self.unit.trim().is_empty() {
            String::new()
        } else {
            format!(" / {}", self.unit.trim())
        };

        if self.description.trim().is_empty() {
            format!("{} — {range}{unit}", self.title)
        } else {
            format!("{} ({}) — {range}{unit}", self.title, self.description)
        }
    }
}

/// Все строки прайса, включая скрытые: админка должна их видеть.
pub async fn load_all(pool: &SqlitePool) -> AppResult<Vec<PriceItem>> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, i64, i64, String)>(
        r#"SELECT slug, title, description, price_from, price_to, unit, sort, is_active, updated_at
           FROM price_items ORDER BY sort, slug"#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(slug, title, description, price_from, price_to, unit, sort, is_active, updated_at)| {
            PriceItem {
                slug,
                title,
                description,
                price_from,
                price_to,
                unit,
                sort,
                is_active: is_active != 0,
                updated_at,
            }
        })
        .collect())
}

/// Только активные строки — для сайта и MCP.
pub async fn load_active(pool: &SqlitePool) -> AppResult<Vec<PriceItem>> {
    Ok(load_all(pool).await?.into_iter().filter(|item| item.is_active).collect())
}

/// Полная замена прайса: админка присылает весь список, сервер пишет его целиком.
///
/// Удаление и вставка — в одной транзакции: половина прайса не должна попасть
/// в ответы ассистента, пока вторая ещё пишется.
pub async fn replace_all(
    pool: &SqlitePool,
    items: &[PriceItem],
    updated_at: &str,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM price_items").execute(&mut *tx).await?;

    for (index, item) in items.iter().enumerate() {
        sqlx::query(
            r#"INSERT INTO price_items
               (slug, title, description, price_from, price_to, unit, sort, is_active, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&item.slug)
        .bind(&item.title)
        .bind(&item.description)
        .bind(&item.price_from)
        .bind(&item.price_to)
        .bind(&item.unit)
        .bind(if item.sort == 0 { index as i64 } else { item.sort })
        .bind(if item.is_active { 1 } else { 0 })
        .bind(updated_at)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Ключ в `site_settings` с отпечатком каталога услуг.
const CATALOG_HASH_KEY: &str = "price_catalog_hash";

/// Версия стартового прайса. Меняется, когда набор задач и цен пересобран:
/// иначе существующая база не увидит новые строки, ведь отпечаток услуг может
/// остаться прежним.
const PRICE_SEED_KEY: &str = "price_seed_version";
const PRICE_SEED_VERSION: &str = "2026-10-30-task-book-v1";

/// Синхронизация прайса с каталогом услуг.
///
/// Прайс — не копия услуг, а отдельная таблица, которую владелец правит в
/// админке. Но когда каталог меняется целиком (новая услуга, переименование,
/// другие цены), старые строки начинают врать. Поэтому при старте сравниваем
/// отпечаток каталога: изменился — пересобираем прайс из услуг; не изменился —
/// оставляем ручные правки владельца нетронутыми.
pub async fn sync_with_services(pool: &SqlitePool, updated_at: &str) -> AppResult<bool> {
    let content = crate::memory::content::SiteContent::get();
    let hash = catalog_hash(&content.services);

    let stored: Option<String> =
        sqlx::query_scalar("SELECT value FROM site_settings WHERE key = ?")
            .bind(CATALOG_HASH_KEY)
            .fetch_optional(pool)
            .await?;
    let stored_seed: Option<String> =
        sqlx::query_scalar("SELECT value FROM site_settings WHERE key = ?")
            .bind(PRICE_SEED_KEY)
            .fetch_optional(pool)
            .await?;

    if stored.as_deref() == Some(hash.as_str())
        && stored_seed.as_deref() == Some(PRICE_SEED_VERSION)
    {
        return Ok(false);
    }

    // Стартовый прайс: базовый формат каждой услуги плюс задачи из каталога.
    // Ручные правки владельца сохраняются, пока не изменился каталог услуг
    // или версия стартового набора.
    let items = price_catalog::build(&content.services, updated_at);
    replace_all(pool, &items, updated_at).await?;

    for (key, value) in [
        (CATALOG_HASH_KEY, hash.as_str()),
        (PRICE_SEED_KEY, PRICE_SEED_VERSION),
    ] {
        sqlx::query(
            r#"INSERT INTO site_settings (key, value, updated_at) VALUES (?, ?, ?)
               ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at"#,
        )
        .bind(key)
        .bind(value)
        .bind(updated_at)
        .execute(pool)
        .await?;
    }

    Ok(true)
}

/// Отпечаток каталога: ключ, название и цена каждой услуги.
///
/// FNV-1a, а не криптография: нужен только признак «каталог изменился», а не
/// защита от подделки.
fn catalog_hash(services: &[crate::memory::content::Service]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for service in services {
        for part in [
            service.slug.as_str(),
            service.title.as_str(),
            service.price_from.as_deref().unwrap_or(""),
            service.price_to.as_deref().unwrap_or(""),
        ] {
            for byte in part.as_bytes() {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x100000001b3);
            }
            hash ^= 0xff;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::init_memory_pool;

    fn item(slug: &str, from: &str, to: &str) -> PriceItem {
        PriceItem {
            slug: slug.into(),
            title: "Проверка".into(),
            description: String::new(),
            price_from: from.into(),
            price_to: to.into(),
            unit: "проект".into(),
            sort: 0,
            is_active: true,
            updated_at: String::new(),
        }
    }

    #[tokio::test]
    async fn replace_then_load_roundtrips() {
        let pool = init_memory_pool().await.unwrap();
        replace_all(&pool, &[item("a", "100", "200"), item("b", "300", "")], "now")
            .await
            .unwrap();

        let loaded = load_all(&pool).await.unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].slug, "a");

        replace_all(&pool, &[item("c", "", "500")], "later").await.unwrap();
        let loaded = load_all(&pool).await.unwrap();
        assert_eq!(loaded.len(), 1, "замена должна удалять старые строки");
        assert_eq!(loaded[0].updated_at, "later");
    }

    #[tokio::test]
    async fn hidden_rows_are_skipped_for_public() {
        let pool = init_memory_pool().await.unwrap();
        let mut hidden = item("hidden", "1", "2");
        hidden.is_active = false;
        replace_all(&pool, &[item("shown", "1", "2"), hidden], "now")
            .await
            .unwrap();

        assert_eq!(load_all(&pool).await.unwrap().len(), 2);
        let active = load_active(&pool).await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].slug, "shown");
    }

    #[test]
    fn line_formats_ranges() {
        assert_eq!(item("a", "100", "200").line(), "Проверка — 100 – 200 / проект");
        assert_eq!(item("a", "100", "").line(), "Проверка — от 100 / проект");
        assert_eq!(item("a", "", "").line(), "Проверка — по запросу / проект");
    }
}

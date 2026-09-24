//! Статьи блога: доменная модель, валидация, публикация и outbox для n8n.
//!
//! # Слои
//!
//! * [`crate::storage::article`] знает только колонки таблиц.
//! * Этот модуль знает предметную область: статусы, slug, правила публикации.
//! * [`PublishedArticles`] — снимок опубликованного в памяти процесса. Он нужен
//!   потому, что Leptos рендерит SSR-страницы **синхронно**: компонент не может
//!   сходить в SQLite за списком статей.
//!
//! # Архитектура под кросспостинг (n8n)
//!
//! Все изменения опубликованных статей порождают событие в таблице
//! `article_events` (transactional outbox). Событие пишется **в одной
//! транзакции** с самой статьёй, поэтому состояния «опубликовано, но событие
//! потерялось» не существует по построению.
//!
//! Читатель (n8n) забирает `pending`-события через `GET /api/integrations/outbox`
//! и подтверждает их через `POST /api/integrations/outbox/ack`. Модель chosen
//! именно pull: сайту не нужно знать адрес n8n, а повторный прогон пайплайна
//! безопасен — ack идемпотентен, а события попадают в `failed` после
//! [`crate::storage::article::MAX_EVENT_ATTEMPTS`] неудач.
//!
//! Push-доставка (n8n Webhook-нода) добавляется поверх этой же схемы: в
//! `article_events` уже есть `attempts` и `next_attempt_at`, поэтому ретраи с
//! backoff не потребуют миграции.

use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::storage::article as repo;
use crate::utils::markdown;

/// Максимальная длина заголовка статьи.
pub const TITLE_MAX_CHARS: usize = 200;
/// Максимальная длина краткого описания.
pub const SUMMARY_MAX_CHARS: usize = 500;
/// Максимальный размер тела статьи в символах (~200 КБ текста).
pub const BODY_MAX_CHARS: usize = 200_000;
/// Максимальная длина slug.
pub const SLUG_MAX_CHARS: usize = 120;
/// Сколько тегов можно повесить на статью.
pub const TAGS_MAX: usize = 10;
/// Максимальная длина одного тега.
pub const TAG_MAX_CHARS: usize = 40;
/// Сколько статей отдавать за один запрос списка по умолчанию.
pub const LIST_DEFAULT_LIMIT: i64 = 50;
/// Верхняя граница `limit` — защита от запроса «дай всё» по API.
pub const LIST_MAX_LIMIT: i64 = 200;

/// Статус статьи.
///
/// `archived` — «снята с публикации, но не удалена»: материал остаётся в
/// админке, однако на сайте его нет.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArticleStatus {
    Draft,
    Published,
    Archived,
}

impl ArticleStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Archived => "archived",
        }
    }

    /// Разбирает статус из БД/JSON. Регистр не важен.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "draft" => Some(Self::Draft),
            "published" => Some(Self::Published),
            "archived" => Some(Self::Archived),
            _ => None,
        }
    }

    /// Видна ли статья посетителям.
    pub fn is_public(self) -> bool {
        matches!(self, Self::Published)
    }
}

impl std::fmt::Display for ArticleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Полная статья (с телом).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Article {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub body_markdown: String,
    pub cover_image_url: Option<String>,
    pub tags: Vec<String>,
    pub status: ArticleStatus,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub source: String,
    pub external_id: Option<String>,
    pub canonical_url: Option<String>,
}

/// Статья без тела — для списков, RSS и админ-таблицы.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ArticleHead {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub cover_image_url: Option<String>,
    pub tags: Vec<String>,
    pub status: ArticleStatus,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub source: String,
    pub canonical_url: Option<String>,
    /// Оценка времени чтения в минутах.
    pub reading_time_minutes: u32,
}

impl Article {
    /// Относительный адрес статьи на сайте.
    pub fn url_path(&self) -> String {
        format!("/blog/{}", self.slug)
    }

    /// Тело статьи, отрендеренное в безопасный HTML.
    pub fn html(&self) -> String {
        markdown::render(&self.body_markdown)
    }

    /// Время чтения в минутах.
    pub fn reading_time_minutes(&self) -> u32 {
        markdown::reading_time_minutes(&self.body_markdown)
    }

    /// Описание для `<meta name="description">`: краткое описание, а если его
    /// нет — первый фрагмент текста.
    pub fn description(&self) -> String {
        if !self.summary.trim().is_empty() {
            return self.summary.trim().to_string();
        }
        markdown::excerpt(&self.body_markdown, 180)
    }

    /// Отбрасывает тело — для списков.
    pub fn head(&self) -> ArticleHead {
        ArticleHead {
            id: self.id,
            slug: self.slug.clone(),
            title: self.title.clone(),
            summary: self.summary.clone(),
            cover_image_url: self.cover_image_url.clone(),
            tags: self.tags.clone(),
            status: self.status,
            published_at: self.published_at.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            source: self.source.clone(),
            canonical_url: self.canonical_url.clone(),
            reading_time_minutes: self.reading_time_minutes(),
        }
    }
}

impl ArticleHead {
    /// Относительный адрес статьи на сайте.
    pub fn url_path(&self) -> String {
        format!("/blog/{}", self.slug)
    }
}

/// Входные данные статьи из админки (или, в будущем, от API-импорта).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ArticleInput {
    pub title: String,
    /// Пусто/`None` — slug сгенерируется из заголовка.
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub body_markdown: String,
    #[serde(default)]
    pub cover_image_url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub status: Option<ArticleStatus>,
    #[serde(default)]
    pub canonical_url: Option<String>,
}

/// Проверенный и нормализованный ввод — то, что уходит в хранилище.
#[derive(Debug, Clone)]
struct Validated {
    title: String,
    slug_base: String,
    summary: String,
    body: String,
    cover: Option<String>,
    tags: Vec<String>,
    status: ArticleStatus,
    canonical: Option<String>,
}

/// Снимок опубликованных статей в памяти процесса.
///
/// Leptos-SSR рендерит страницы синхронно, поэтому список блога и страница
/// статьи читаются отсюда. Снимок пересобирается из БД после каждой мутации
/// ([`ArticleService::reload_published`]) и один раз при старте.
pub struct PublishedArticles;

static PUBLISHED: RwLock<Vec<Article>> = RwLock::new(Vec::new());

impl PublishedArticles {
    /// Публикует новый снимок (свежие сверху).
    pub fn set(articles: Vec<Article>) {
        let mut guard = PUBLISHED.write().unwrap_or_else(|e| e.into_inner());
        *guard = articles;
    }

    /// Список опубликованных статей, свежие сверху.
    pub fn list() -> Vec<Article> {
        let guard = PUBLISHED.read().unwrap_or_else(|e| e.into_inner());
        guard.clone()
    }

    /// Шапки опубликованных статей (без тела) — для списка блога.
    pub fn heads() -> Vec<ArticleHead> {
        let guard = PUBLISHED.read().unwrap_or_else(|e| e.into_inner());
        guard.iter().map(Article::head).collect()
    }

    /// Опубликованная статья по slug.
    pub fn find(slug: &str) -> Option<Article> {
        let guard = PUBLISHED.read().unwrap_or_else(|e| e.into_inner());
        guard.iter().find(|a| a.slug == slug).cloned()
    }

    /// Сколько статей опубликовано.
    pub fn count() -> usize {
        let guard = PUBLISHED.read().unwrap_or_else(|e| e.into_inner());
        guard.len()
    }

    /// Сбрасывает снимок. Только для тестов: они делят процесс, и статьи,
    /// опубликованные одним тестом, не должны утекать в другой.
    #[doc(hidden)]
    pub fn reset_global() {
        let mut guard = PUBLISHED.write().unwrap_or_else(|e| e.into_inner());
        guard.clear();
    }
}

/// Сервис статей: единственная точка записи.
#[derive(Clone)]
pub struct ArticleService {
    db: SqlitePool,
    /// Публичный адрес сайта — из него собираются абсолютные ссылки в
    /// событиях для n8n (и далее в RSS/sitemap).
    public_url: String,
}

impl ArticleService {
    pub fn new(db: SqlitePool, public_url: impl Into<String>) -> Self {
        Self {
            db,
            public_url: public_url.into().trim_end_matches('/').to_string(),
        }
    }

    /// Публичный адрес сайта без завершающего слэша.
    pub fn public_url(&self) -> &str {
        &self.public_url
    }

    /// Список статей для админки: любые статусы, свежие сверху.
    pub async fn list(
        &self,
        status: Option<ArticleStatus>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> AppResult<Vec<ArticleHead>> {
        let limit = clamp_limit(limit);
        let offset = offset.unwrap_or(0).max(0);
        let rows = repo::list(&self.db, status.map(|s| s.as_str()), limit, offset).await?;

        let mut heads = Vec::with_capacity(rows.len());
        for row in &rows {
            heads.push(row_to_article(row)?.head());
        }
        Ok(heads)
    }

    /// Статья по id.
    pub async fn get(&self, id: i64) -> AppResult<Article> {
        row_to_article(&repo::get(&self.db, id).await?)
    }

    /// Сколько статей в каждом статусе.
    pub async fn count(&self, status: Option<ArticleStatus>) -> AppResult<i64> {
        repo::count(&self.db, status.map(|s| s.as_str())).await
    }

    /// Создаёт статью.
    pub async fn create(&self, input: ArticleInput) -> AppResult<Article> {
        let validated = self.validate(input, None, ArticleStatus::Draft).await?;
        let published_at = validated
            .status
            .is_public()
            .then(|| chrono::Utc::now().to_rfc3339());

        // Событие создаётся только для того, что реально попадает на сайт:
        // черновик кросспостить нечего.
        let event_type = validated
            .status
            .is_public()
            .then(|| "article.published".to_string());
        let public_url = self.public_url.clone();

        let row = repo::insert(
            &self.db,
            repo::NewArticle {
                slug: validated.slug_base,
                title: validated.title,
                summary: validated.summary,
                body_markdown: validated.body,
                cover_image_url: validated.cover,
                tags: tags_to_json(&validated.tags)?,
                status: validated.status.as_str().to_string(),
                published_at,
                source: "admin".to_string(),
                external_id: None,
                canonical_url: validated.canonical,
            },
            event_type.map(|event_type| {
                (event_type, move |row: &repo::ArticleRow| {
                    event_payload(&public_url, row)
                })
            }),
        )
        .await?;

        let article = row_to_article(&row)?;
        self.reload_published().await?;
        tracing::info!("статья создана: #{} «{}» ({})", article.id, article.title, article.status);
        Ok(article)
    }

    /// Полностью обновляет статью и, если нужно, порождает событие.
    ///
    /// Событие зависит от перехода статуса:
    /// * был не опубликован → стал опубликован: `article.published`
    /// * был опубликован → остался: `article.updated`
    /// * был опубликован → снят: `article.unpublished`
    /// * черновик → черновик: события нет (кросспостить нечего)
    pub async fn update(&self, id: i64, input: ArticleInput) -> AppResult<Article> {
        let existing = self.get(id).await?;
        let validated = self
            .validate(input, Some(id), existing.status)
            .await?;

        // Дата публикации — момент, когда статья впервые стала публичной в
        // текущей «публикации». Снятие с публикации её обнуляет: опубликовать
        // заново и увидеть старую дату было бы неожиданно.
        let published_at = if validated.status.is_public() {
            existing.published_at.clone().or_else(|| Some(chrono::Utc::now().to_rfc3339()))
        } else {
            None
        };

        let event_type = transition_event(existing.status, validated.status);
        let public_url = self.public_url.clone();

        let row = repo::update(
            &self.db,
            id,
            repo::ArticlePatch {
                slug: validated.slug_base,
                title: validated.title,
                summary: validated.summary,
                body_markdown: validated.body,
                cover_image_url: validated.cover,
                tags: tags_to_json(&validated.tags)?,
                status: validated.status.as_str().to_string(),
                published_at,
                canonical_url: validated.canonical,
            },
            event_type.map(|event_type| {
                (event_type, move |row: &repo::ArticleRow| {
                    event_payload(&public_url, row)
                })
            }),
        )
        .await?;

        let article = row_to_article(&row)?;
        self.reload_published().await?;
        tracing::info!("статья обновлена: #{} «{}» ({})", article.id, article.title, article.status);
        Ok(article)
    }

    /// Меняет только статус, не трогая текст.
    pub async fn set_status(&self, id: i64, status: ArticleStatus) -> AppResult<Article> {
        let existing = self.get(id).await?;
        self.update(
            id,
            ArticleInput {
                title: existing.title,
                slug: Some(existing.slug),
                summary: existing.summary,
                body_markdown: existing.body_markdown,
                cover_image_url: existing.cover_image_url,
                tags: existing.tags,
                status: Some(status),
                canonical_url: existing.canonical_url,
            },
        )
        .await
    }

    /// Удаляет статью. Событие создаётся, только если статья была на сайте.
    pub async fn delete(&self, id: i64) -> AppResult<Article> {
        let existing = self.get(id).await?;
        let event_type = existing
            .status
            .is_public()
            .then(|| "article.deleted".to_string());
        let public_url = self.public_url.clone();

        let row = repo::delete(
            &self.db,
            id,
            event_type.map(|event_type| {
                (event_type, move |row: &repo::ArticleRow| {
                    event_payload(&public_url, row)
                })
            }),
        )
        .await?
        .ok_or_else(|| AppError::NotFound(format!("article #{id}")))?;

        let article = row_to_article(&row)?;
        self.reload_published().await?;
        tracing::info!("статья удалена: #{} «{}»", article.id, article.title);
        Ok(article)
    }

    /// Перечитывает опубликованные статьи из БД в [`PublishedArticles`].
    pub async fn reload_published(&self) -> AppResult<()> {
        let rows = repo::list_published_full(&self.db).await?;
        let articles = rows
            .iter()
            .map(row_to_article)
            .collect::<AppResult<Vec<Article>>>()?;
        tracing::debug!("опубликованных статей: {}", articles.len());
        PublishedArticles::set(articles);
        Ok(())
    }

    /// Проверяет ввод и подбирает свободный slug.
    ///
    /// `exclude_id` — id обновляемой статьи: свой же slug не считается занятым.
    /// `default_status` применяется, если статус не передан явно: при создании
    /// это черновик, при обновлении — текущий статус. Иначе PATCH без поля
    /// `status` молча снимал бы статью с публикации.
    async fn validate(
        &self,
        input: ArticleInput,
        exclude_id: Option<i64>,
        default_status: ArticleStatus,
    ) -> AppResult<Validated> {
        let title = input.title.trim().to_string();
        if title.is_empty() {
            return Err(AppError::Validation("заголовок статьи не может быть пустым".into()));
        }
        if title.chars().count() > TITLE_MAX_CHARS {
            return Err(AppError::Validation(format!(
                "заголовок длиннее {TITLE_MAX_CHARS} символов"
            )));
        }

        let body = input.body_markdown.trim().to_string();
        if body.is_empty() {
            return Err(AppError::Validation("текст статьи не может быть пустым".into()));
        }
        if body.chars().count() > BODY_MAX_CHARS {
            return Err(AppError::Validation(format!(
                "текст статьи длиннее {BODY_MAX_CHARS} символов"
            )));
        }

        let summary = input.summary.trim().to_string();
        if summary.chars().count() > SUMMARY_MAX_CHARS {
            return Err(AppError::Validation(format!(
                "краткое описание длиннее {SUMMARY_MAX_CHARS} символов"
            )));
        }

        let tags = normalize_tags(input.tags)?;
        let cover = normalize_url(input.cover_image_url, "обложка")?;
        let canonical = normalize_url(input.canonical_url, "канонический адрес")?;

        // Явно заданный slug уважаем как есть (нормализовав), иначе выводим из
        // заголовка — так админке не нужно придумывать транслит вручную.
        let requested = input
            .slug
            .as_deref()
            .map(slugify)
            .filter(|s| !s.is_empty());
        let base = match requested {
            Some(slug) => slug,
            None => {
                let derived = slugify(&title);
                if derived.is_empty() {
                    // Заголовок целиком из символов, которые не транслитерируются
                    // (например, эмодзи) — даём предсказуемую основу.
                    "article".to_string()
                } else {
                    derived
                }
            }
        };

        let exclude = exclude_id;
        let slug = self.unique_slug(&base, exclude).await?;

        Ok(Validated {
            title,
            slug_base: slug,
            summary,
            body,
            cover,
            tags,
            status: input.status.unwrap_or(default_status),
            canonical,
        })
    }

    /// Подбирает свободный slug: `base`, `base-2`, `base-3`, …
    async fn unique_slug(&self, base: &str, exclude_id: Option<i64>) -> AppResult<String> {
        if !repo::slug_taken(&self.db, base, exclude_id).await? {
            return Ok(base.to_string());
        }

        // 100 попыток — с большим запасом: коллизии возможны только при
        // массовом импорте однотипных заголовков.
        for n in 2..=100 {
            let candidate = format!("{base}-{n}");
            if !repo::slug_taken(&self.db, &candidate, exclude_id).await? {
                return Ok(candidate);
            }
        }

        Err(AppError::Validation(
            "не удалось подобрать свободный адрес статьи".into(),
        ))
    }
}

/// Событие перехода между статусами (или `None`, если событие не нужно).
fn transition_event(from: ArticleStatus, to: ArticleStatus) -> Option<String> {
    match (from.is_public(), to.is_public()) {
        (false, true) => Some("article.published".to_string()),
        (true, true) => Some("article.updated".to_string()),
        (true, false) => Some("article.unpublished".to_string()),
        (false, false) => None,
    }
}

/// Собирает payload события — снимок статьи на момент изменения.
///
/// Именно снимок, а не ссылка на id: к моменту, когда n8n заберёт событие,
/// статью могли удалить или переписать, а кросспостить нужно то, что было
/// опубликовано.
fn event_payload(public_url: &str, row: &repo::ArticleRow) -> String {
    let tags: Vec<String> = serde_json::from_str(&row.tags).unwrap_or_default();
    let payload = serde_json::json!({
        "article": {
            "id": row.id,
            "slug": row.slug,
            "url": format!("{public_url}/blog/{}", row.slug),
            "title": row.title,
            "summary": row.summary,
            "body_markdown": row.body_markdown,
            "cover_image_url": row.cover_image_url,
            "tags": tags,
            "status": row.status,
            "published_at": row.published_at,
            "updated_at": row.updated_at,
            "reading_time_minutes": markdown::reading_time_minutes(&row.body_markdown),
            "canonical_url": row.canonical_url,
        }
    });
    // Сериализация `serde_json::Value` в строку не может упасть.
    payload.to_string()
}

/// Приводит строку из БД к доменной модели.
fn row_to_article(row: &repo::ArticleRow) -> AppResult<Article> {
    // Повреждённый JSON тегов не должен ронять страницу: показываем статью
    // без тегов и пишем предупреждение.
    let tags: Vec<String> = match serde_json::from_str(&row.tags) {
        Ok(tags) => tags,
        Err(e) => {
            tracing::warn!("article #{}: повреждённый JSON тегов, игнорируем: {e}", row.id);
            Vec::new()
        }
    };

    let status = ArticleStatus::parse(&row.status).ok_or_else(|| {
        AppError::Internal(format!(
            "article #{}: неизвестный статус «{}»",
            row.id, row.status
        ))
    })?;

    Ok(Article {
        id: row.id,
        slug: row.slug.clone(),
        title: row.title.clone(),
        summary: row.summary.clone(),
        body_markdown: row.body_markdown.clone(),
        cover_image_url: row.cover_image_url.clone(),
        tags,
        status,
        published_at: row.published_at.clone(),
        created_at: row.created_at.clone(),
        updated_at: row.updated_at.clone(),
        source: row.source.clone(),
        external_id: row.external_id.clone(),
        canonical_url: row.canonical_url.clone(),
    })
}

fn tags_to_json(tags: &[String]) -> AppResult<String> {
    Ok(serde_json::to_string(tags)?)
}

/// Нормализует теги: обрезает пробелы, убирает пустые и дубликаты
/// (без учёта регистра), проверяет количество и длину.
fn normalize_tags(tags: Vec<String>) -> AppResult<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for tag in tags {
        let tag = tag.trim().trim_start_matches('#').trim().to_string();
        if tag.is_empty() {
            continue;
        }
        if tag.chars().count() > TAG_MAX_CHARS {
            return Err(AppError::Validation(format!(
                "тег «{tag}» длиннее {TAG_MAX_CHARS} символов"
            )));
        }
        if out.iter().any(|t| t.eq_ignore_ascii_case(&tag)) {
            continue;
        }
        out.push(tag);
    }

    if out.len() > TAGS_MAX {
        return Err(AppError::Validation(format!(
            "можно указать не больше {TAGS_MAX} тегов"
        )));
    }
    Ok(out)
}

/// Проверяет URL обложки/канонического адреса.
///
/// Разрешены абсолютные `http(s)://` и локальные пути (`/img/...`): картинки
/// удобно держать на своём домене, а не только на внешнем хостинге.
fn normalize_url(value: Option<String>, field: &str) -> AppResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_string();
    if value.is_empty() {
        return Ok(None);
    }

    let ok = value.starts_with("https://") || value.starts_with("http://") || value.starts_with('/');
    if !ok {
        return Err(AppError::Validation(format!(
            "{field}: нужен адрес http(s):// или путь, начинающийся с «/»"
        )));
    }
    if value.chars().count() > 500 {
        return Err(AppError::Validation(format!("{field}: адрес слишком длинный")));
    }
    // Отсекаем `//example.com` — это protocol-relative ссылка на чужой сайт,
    // замаскированная под локальный путь.
    if value.starts_with("//") {
        return Err(AppError::Validation(format!("{field}: адрес не может начинаться с «//»")));
    }

    Ok(Some(value))
}

/// Ограничивает `limit` разумными рамками.
fn clamp_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(LIST_DEFAULT_LIMIT).clamp(1, LIST_MAX_LIMIT)
}

/// Транслитерация заголовка в slug.
///
/// Кириллица → латиница (таблица ближе к ГОСТ, но с приоритетом читаемости:
/// `ё` → `e`, `й` → `y`, `ь`/`ъ` отбрасываются), остальное — в дефисы.
pub fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut pending_dash = false;

    for ch in input.chars() {
        // `to_ascii_lowercase` не трогает кириллицу, поэтому нужна
        // Unicode-версия: без неё «Одна статья» теряла первую букву.
        for lower in ch.to_lowercase() {
            // Латиница и цифры проходят как есть — это основной случай для
            // технических заголовков вроде «MCP-серверы и AI Skills».
            let mapped = if lower.is_ascii_alphanumeric() {
                Some(lower.to_string())
            } else {
                transliterate(lower).map(str::to_string)
            };

            match mapped {
                // Разделитель (пробел, знак, эмодзи) — откладываем дефис.
                None => pending_dash = !out.is_empty(),
                // Буква, которая не даёт латинских символов (`ь`, `ъ`):
                // просто выпадает и дефис не создаёт, иначе «статья»
                // превратилась бы в «stat-ya».
                Some(m) if m.is_empty() => {}
                Some(m) => {
                    // Дефис ставится один на серию разделителей и не в начале.
                    if pending_dash && !out.is_empty() {
                        out.push('-');
                    }
                    pending_dash = false;
                    // `transliterate` возвращает только ASCII-буквы и цифры.
                    out.push_str(&m);
                }
            }
        }
    }

    // Обрезаем по границе слова, чтобы slug не заканчивался огрызком.
    if out.len() > SLUG_MAX_CHARS {
        out.truncate(SLUG_MAX_CHARS);
        if let Some(idx) = out.rfind('-') {
            out.truncate(idx);
        }
    }
    out.trim_matches('-').to_string()
}

/// Один символ → его латинское представление.
///
/// `None` — символ-разделитель (станет дефисом). `Some("")` — буква, которая
/// не даёт латинских символов (`ь`, `ъ`): она выпадает, не создавая дефис.
/// ASCII сюда не попадает: `slugify` обрабатывает его раньше.
fn transliterate(ch: char) -> Option<&'static str> {
    let mapped = match ch {
        'а' => "a", 'б' => "b", 'в' => "v", 'г' => "g", 'д' => "d",
        'е' => "e", 'ё' => "e", 'ж' => "zh", 'з' => "z", 'и' => "i",
        'й' => "y", 'к' => "k", 'л' => "l", 'м' => "m", 'н' => "n",
        'о' => "o", 'п' => "p", 'р' => "r", 'с' => "s", 'т' => "t",
        'у' => "u", 'ф' => "f", 'х' => "h", 'ц' => "ts", 'ч' => "ch",
        'ш' => "sh", 'щ' => "sch", 'ы' => "y",
        'э' => "e", 'ю' => "yu", 'я' => "ya",
        // Украинские буквы: сайт русскоязычный, но гостей из СНГ хватает.
        'і' => "i", 'ї' => "yi", 'є' => "ye", 'ґ' => "g",
        // Твёрдый и мягкий знаки не читаются как отдельный звук.
        'ь' | 'ъ' => "",
        // Всё остальное (эмодзи, CJK, знаки) — разделитель.
        _ => return None,
    };
    Some(mapped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::init_memory_pool;

    fn service(pool: SqlitePool) -> ArticleService {
        ArticleService::new(pool, "https://inteli-dev.ru")
    }

    fn input(title: &str, body: &str) -> ArticleInput {
        ArticleInput {
            title: title.into(),
            body_markdown: body.into(),
            ..Default::default()
        }
    }

    #[test]
    fn slugify_transliterates_cyrillic() {
        assert_eq!(slugify("Привет, мир!"), "privet-mir");
        assert_eq!(slugify("Ёлка и ёжик"), "elka-i-ezhik");
        assert_eq!(slugify("MCP-серверы и AI Skills"), "mcp-servery-i-ai-skills");
        assert_eq!(slugify("Щука, чай, юла"), "schuka-chay-yula");
        // Мягкий и твёрдый знаки выпадают, не создавая дефис.
        assert_eq!(slugify("Статья о съезде"), "statya-o-sezde");
        assert_eq!(slugify("Нью-Йорк"), "nyu-york");
    }

    #[test]
    fn slugify_collapses_separators_and_trims() {
        assert_eq!(slugify("  Много    пробелов  "), "mnogo-probelov");
        assert_eq!(slugify("---уже---дефисы---"), "uzhe-defisy");
        assert_eq!(slugify("!!!"), "");
    }

    #[test]
    fn slugify_respects_max_length() {
        let long = "слово ".repeat(60);
        let slug = slugify(&long);
        assert!(slug.len() <= SLUG_MAX_CHARS, "slug слишком длинный: {slug}");
        assert!(!slug.ends_with('-'));
    }

    #[test]
    fn status_roundtrips_through_strings() {
        for status in [
            ArticleStatus::Draft,
            ArticleStatus::Published,
            ArticleStatus::Archived,
        ] {
            assert_eq!(ArticleStatus::parse(status.as_str()), Some(status));
        }
        assert_eq!(ArticleStatus::parse("PUBLISHED"), Some(ArticleStatus::Published));
        assert_eq!(ArticleStatus::parse("bogus"), None);
        assert!(ArticleStatus::Published.is_public());
        assert!(!ArticleStatus::Draft.is_public());
    }

    #[test]
    fn transition_event_covers_all_edges() {
        use ArticleStatus::*;
        assert_eq!(transition_event(Draft, Published).as_deref(), Some("article.published"));
        assert_eq!(transition_event(Published, Published).as_deref(), Some("article.updated"));
        assert_eq!(transition_event(Published, Draft).as_deref(), Some("article.unpublished"));
        assert_eq!(transition_event(Published, Archived).as_deref(), Some("article.unpublished"));
        // Ни черновик, ни архив на сайте не видны — кросспостить нечего.
        assert!(transition_event(Draft, Draft).is_none());
        assert!(transition_event(Draft, Archived).is_none());
        assert!(transition_event(Archived, Archived).is_none());
    }

    #[test]
    fn tags_are_normalized() {
        let tags = normalize_tags(vec![
            " Rust ".into(),
            "#rust".into(),
            "RUST".into(),
            "".into(),
            "AI".into(),
        ])
        .unwrap();
        assert_eq!(tags, vec!["Rust", "AI"]);
    }

    #[test]
    fn too_many_tags_are_rejected() {
        let many: Vec<String> = (0..TAGS_MAX + 1).map(|i| format!("tag{i}")).collect();
        assert!(matches!(normalize_tags(many), Err(AppError::Validation(_))));
    }

    #[test]
    fn urls_are_validated() {
        assert_eq!(normalize_url(None, "обложка").unwrap(), None);
        assert_eq!(normalize_url(Some("  ".into()), "обложка").unwrap(), None);
        assert_eq!(
            normalize_url(Some("/img/a.png".into()), "обложка").unwrap(),
            Some("/img/a.png".into())
        );
        assert!(normalize_url(Some("https://x.ru/a.png".into()), "обложка").is_ok());
        assert!(normalize_url(Some("javascript:alert(1)".into()), "обложка").is_err());
        // Protocol-relative ссылка ведёт на чужой домен — отклоняем.
        assert!(normalize_url(Some("//evil.com/a.png".into()), "обложка").is_err());
    }

    #[test]
    fn limit_is_clamped() {
        assert_eq!(clamp_limit(None), LIST_DEFAULT_LIMIT);
        assert_eq!(clamp_limit(Some(0)), 1);
        assert_eq!(clamp_limit(Some(10_000)), LIST_MAX_LIMIT);
        assert_eq!(clamp_limit(Some(10)), 10);
    }

    #[tokio::test]
    async fn create_derives_slug_from_title() {
        let pool = init_memory_pool().await.unwrap();
        let svc = service(pool);

        let article = svc.create(input("Как я собирал AI-сервер", "Текст")).await.unwrap();
        assert_eq!(article.slug, "kak-ya-sobiral-ai-server");
        assert_eq!(article.status, ArticleStatus::Draft);
        assert_eq!(article.source, "admin");
        assert!(article.published_at.is_none());
    }

    #[tokio::test]
    async fn create_resolves_slug_collisions() {
        let pool = init_memory_pool().await.unwrap();
        let svc = service(pool);

        let a = svc.create(input("Одна статья", "x")).await.unwrap();
        let b = svc.create(input("Одна статья", "x")).await.unwrap();
        assert_eq!(a.slug, "odna-statya");
        assert_eq!(b.slug, "odna-statya-2");
    }

    #[tokio::test]
    async fn create_rejects_empty_title_and_body() {
        let pool = init_memory_pool().await.unwrap();
        let svc = service(pool);

        assert!(matches!(
            svc.create(input("   ", "текст")).await,
            Err(AppError::Validation(_))
        ));
        assert!(matches!(
            svc.create(input("Заголовок", "   ")).await,
            Err(AppError::Validation(_))
        ));
    }

    #[tokio::test]
    async fn create_rejects_overlong_title() {
        let pool = init_memory_pool().await.unwrap();
        let svc = service(pool);
        let long = "а".repeat(TITLE_MAX_CHARS + 1);
        assert!(matches!(
            svc.create(input(&long, "текст")).await,
            Err(AppError::Validation(_))
        ));
    }

    #[tokio::test]
    async fn draft_emits_no_event_but_publishing_does() {
        let pool = init_memory_pool().await.unwrap();
        let svc = service(pool.clone());

        let draft = svc.create(input("Черновик", "текст")).await.unwrap();
        assert_eq!(repo::pending_event_count(&pool).await.unwrap(), 0);

        let published = svc
            .set_status(draft.id, ArticleStatus::Published)
            .await
            .unwrap();
        assert!(published.published_at.is_some());

        let events = repo::list_events(&pool, "pending", 10).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "article.published");
        // Payload — снимок: n8n получит текст, даже если статью потом удалят.
        let payload: serde_json::Value = serde_json::from_str(&events[0].payload).unwrap();
        assert_eq!(payload["article"]["slug"], "chernovik");
        assert_eq!(payload["article"]["url"], "https://inteli-dev.ru/blog/chernovik");
        assert_eq!(payload["article"]["body_markdown"], "текст");
    }

    #[tokio::test]
    async fn editing_a_published_article_emits_updated() {
        let pool = init_memory_pool().await.unwrap();
        let svc = service(pool.clone());

        let mut data = input("Опубликованная", "первая версия");
        data.status = Some(ArticleStatus::Published);
        let article = svc.create(data).await.unwrap();
        assert_eq!(repo::list_events(&pool, "pending", 10).await.unwrap()[0].event_type, "article.published");

        let mut edit = input("Опубликованная", "вторая версия");
        edit.status = Some(ArticleStatus::Published);
        svc.update(article.id, edit).await.unwrap();

        let events = repo::list_events(&pool, "pending", 10).await.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].event_type, "article.updated");
    }

    #[tokio::test]
    async fn unpublishing_emits_unpublished_and_clears_date() {
        let pool = init_memory_pool().await.unwrap();
        let svc = service(pool.clone());

        let mut data = input("Снять с публикации", "текст");
        data.status = Some(ArticleStatus::Published);
        let article = svc.create(data).await.unwrap();

        let archived = svc.set_status(article.id, ArticleStatus::Archived).await.unwrap();
        assert!(archived.published_at.is_none());

        let events = repo::list_events(&pool, "pending", 10).await.unwrap();
        assert_eq!(events.last().unwrap().event_type, "article.unpublished");
    }

    #[tokio::test]
    async fn deleting_a_published_article_emits_deleted() {
        let pool = init_memory_pool().await.unwrap();
        let svc = service(pool.clone());

        let mut data = input("Удаляемая", "текст");
        data.status = Some(ArticleStatus::Published);
        let article = svc.create(data).await.unwrap();

        let removed = svc.delete(article.id).await.unwrap();
        assert_eq!(removed.slug, "udalyaemaya");

        let events = repo::list_events(&pool, "pending", 10).await.unwrap();
        assert_eq!(events.last().unwrap().event_type, "article.deleted");
    }

    #[tokio::test]
    async fn deleting_a_draft_emits_nothing() {
        let pool = init_memory_pool().await.unwrap();
        let svc = service(pool.clone());

        let draft = svc.create(input("Черновик", "текст")).await.unwrap();
        svc.delete(draft.id).await.unwrap();
        assert_eq!(repo::pending_event_count(&pool).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn update_keeps_own_slug_and_published_date() {
        let pool = init_memory_pool().await.unwrap();
        let svc = service(pool);

        let mut data = input("Стабильная", "текст");
        data.status = Some(ArticleStatus::Published);
        let article = svc.create(data).await.unwrap();
        let first_date = article.published_at.clone().unwrap();

        // slug не передан — выведется из того же заголовка, и это не коллизия
        // с самим собой.
        let mut edit = input("Стабильная", "обновлённый текст");
        edit.status = Some(ArticleStatus::Published);
        let updated = svc.update(article.id, edit).await.unwrap();

        assert_eq!(updated.slug, article.slug);
        assert_eq!(updated.published_at.as_deref(), Some(first_date.as_str()));
    }

    #[tokio::test]
    async fn renaming_regenerates_slug_only_when_not_pinned() {
        let pool = init_memory_pool().await.unwrap();
        let svc = service(pool);

        let article = svc.create(input("Старый заголовок", "текст")).await.unwrap();
        let renamed = svc.update(article.id, input("Новый заголовок", "текст")).await.unwrap();
        assert_eq!(renamed.slug, "novyy-zagolovok");
    }

    #[tokio::test]
    async fn get_missing_article_is_not_found() {
        let pool = init_memory_pool().await.unwrap();
        let svc = service(pool);
        assert!(matches!(svc.get(999).await, Err(AppError::NotFound(_))));
        assert!(matches!(svc.delete(999).await, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn list_returns_heads_without_body() {
        let pool = init_memory_pool().await.unwrap();
        let svc = service(pool);

        let mut data = input("На сайте", &"тело ".repeat(500));
        data.status = Some(ArticleStatus::Published);
        svc.create(data).await.unwrap();
        svc.create(input("Черновик", "тело")).await.unwrap();

        let heads = svc.list(None, None, None).await.unwrap();
        assert_eq!(heads.len(), 2);

        let published = svc
            .list(Some(ArticleStatus::Published), None, None)
            .await
            .unwrap();
        assert_eq!(published.len(), 1);
        assert!(published[0].reading_time_minutes >= 1);
        assert_eq!(svc.count(Some(ArticleStatus::Draft)).await.unwrap(), 1);
    }
}

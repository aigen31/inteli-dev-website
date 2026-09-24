//! Обработчики HTTP API.

use axum::extract::{Path, Query, State};
use axum::http::header::{self, HeaderMap};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::api::{assets, client_ip, ApiError};
use crate::error::AppError;
use crate::services::article::{ArticleInput, ArticleStatus, BODY_MAX_CHARS};
use crate::services::chat::{requires_llm, ChatRequest, ChatResponse};
use crate::services::lead::LeadSubmission;
use crate::services::status::{status_payload, StatusPayload};
use crate::state::AppState;
use crate::storage::lead::Lead;
use crate::utils::ip::hash_ip;

/// Health-check для Docker/K8s (см. docker-build.md).
pub async fn health(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let db_ok = sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();
    let uptime = state.started_at.elapsed().as_secs();
    let status = if db_ok { "healthy" } else { "degraded" };
    Ok(Json(serde_json::json!({
        "status": status,
        "database": db_ok,
        "uptime_seconds": uptime,
        "version": env!("CARGO_PKG_VERSION"),
    })))
}

/// Статус занятости (GET /api/status, /api/status.json).
pub async fn status() -> Json<StatusPayload> {
    Json(status_payload())
}

/// Чат (POST /api/chat).
pub async fn chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, ApiError> {
    let ip_hash = hash_ip(&client_ip(&headers));

    // 1. Технический антифлуд — на все типы запросов, включая preset.
    if !state.rate_limiter.check(&ip_hash) {
        let retry_after = state.config.ratelimit.window_seconds.max(1);
        return Err(AppError::RateLimitExceeded { retry_after }.into());
    }

    // 2. Часовая квота — только на то, что реально тратит токены LLM.
    //    preset/availability/lead_request отдаются из контента бесплатно, а
    //    ответ из кэша не обращается к модели — квоту он не расходует.
    if requires_llm(&req) && !state.chat.is_cached(&req) && !state.hourly_limiter.check(&ip_hash) {
        let retry_after = state.hourly_limiter.retry_after(&ip_hash);
        return Err(AppError::RateLimitExceeded { retry_after }.into());
    }

    let response = state.chat.answer(req, Some(ip_hash)).await?;
    Ok(Json(response))
}

/// Заявка/лид (POST /api/lead).
pub async fn lead(
    State(state): State<AppState>,
    Json(sub): Json<LeadSubmission>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let lead = state.leads.create(sub).await?;
    Ok(Json(serde_json::json!({
        "success": true,
        "lead_id": lead.id,
        "message": "Ваша заявка принята! Я свяжусь с вами в ближайшее время.",
    })))
}

/// Webhook Telegram: обрабатывает callback от inline-кнопок заявок.
pub async fn telegram_webhook(
    State(state): State<AppState>,
    Json(update): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Some(data) = update
        .get("callback_query")
        .and_then(|cb| cb.get("data"))
        .and_then(|d| d.as_str())
    {
        handle_lead_callback(&state, data).await;
    }

    // Telegram ждёт 200 OK независимо от результата обработки.
    StatusCode::OK
}

async fn handle_lead_callback(state: &AppState, data: &str) {
    let parts: Vec<&str> = data.split(':').collect();
    if parts.len() != 3 || parts[0] != "lead" {
        return;
    }

    let status = match parts[1] {
        "convert" => "converted",
        "processing" => "processing",
        _ => return,
    };
    let Ok(id) = parts[2].parse::<i64>() else {
        return;
    };

    if let Err(e) = crate::storage::lead::update_status(&state.db, id, status).await {
        tracing::warn!("telegram callback failed for lead {id}: {e}");
    }
}

/// Webhook бота настроек сайта: владелец меняет состояние сайта из Telegram.
///
/// Защита — заголовок `X-Telegram-Bot-Api-Secret-Token`, который Telegram
/// присылает, если вебхук зарегистрирован с `secret_token`. Без совпадения
/// отвечаем 404: эндпоинт не должен даже подтверждать своё существование.
///
/// Telegram ждёт 200 и повторяет доставку при любой другой ошибке, поэтому
/// ошибки обработки логируются, но наружу не отдаются — иначе один и тот же
/// апдейт будет приходить бесконечно.
pub async fn settings_bot_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(update): Json<serde_json::Value>,
) -> impl IntoResponse {
    if !state.settings_bot.is_active() {
        return StatusCode::NOT_FOUND;
    }

    let presented = headers
        .get(crate::services::settings_bot::SECRET_HEADER)
        .and_then(|v| v.to_str().ok());

    if !state.settings_bot.secret_matches(presented) {
        tracing::warn!("settings bot: отклонён вебхук с неверным секретом");
        return StatusCode::NOT_FOUND;
    }

    state.settings_bot.handle_update(&state.db, &update).await;
    StatusCode::OK
}

// ---------------------------------------------------------------------------
// Admin (token auth)
// ---------------------------------------------------------------------------

fn check_admin(headers: &HeaderMap, token: &str) -> Result<(), ApiError> {
    if token.is_empty() {
        tracing::warn!("admin token is not configured");
        return Err(AppError::Unauthorized.into());
    }
    let presented = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.strip_prefix("Bearer ").unwrap_or(v).to_string());

    if presented.as_deref() == Some(token) {
        Ok(())
    } else {
        Err(AppError::Unauthorized.into())
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
}

/// Сводка для админ-панели.
pub async fn admin_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_admin(&headers, &state.config.admin.token)?;

    let leads_today = crate::storage::lead::count_today(&state.db).await?;
    let chats_today = crate::storage::chat::count_today(&state.db).await?;
    let avg_response_time_ms = crate::storage::chat::avg_response_time_ms(&state.db).await?;
    let lead_sources = crate::storage::lead::source_counts(&state.db).await?;
    let question_types = crate::storage::chat::question_type_counts(&state.db).await?;

    Ok(Json(serde_json::json!({
        "leads_today": leads_today,
        "chats_today": chats_today,
        "avg_response_time_ms": avg_response_time_ms,
        "lead_sources": lead_sources,
        "question_types": question_types,
    })))
}

/// Список лидов.
pub async fn admin_leads(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Lead>>, ApiError> {
    check_admin(&headers, &state.config.admin.token)?;
    let rows = crate::storage::lead::list(&state.db, q.limit.unwrap_or(100)).await?;
    Ok(Json(rows))
}

/// Список чатов.
pub async fn admin_chats(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<crate::storage::chat::ChatEntry>>, ApiError> {
    check_admin(&headers, &state.config.admin.token)?;
    let rows = crate::storage::chat::recent(&state.db, q.limit.unwrap_or(100)).await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct UpdateLeadStatus {
    pub status: String,
}

/// Обновление статуса лида.
pub async fn admin_update_lead(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(body): Json<UpdateLeadStatus>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_admin(&headers, &state.config.admin.token)?;

    const VALID: &[&str] = &["new", "processing", "contacted", "converted", "dismissed"];
    if !VALID.contains(&body.status.as_str()) {
        return Err(AppError::Validation("недопустимый статус лида".into()).into());
    }

    crate::storage::lead::update_status(&state.db, id, &body.status).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

// ---------------------------------------------------------------------------
// Admin: статьи блога
// ---------------------------------------------------------------------------

/// Список статей для админки + счётчики по статусам.
#[derive(Deserialize)]
pub struct ArticleListQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn admin_articles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ArticleListQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_admin(&headers, &state.config.admin.token)?;

    let status = match q.status.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => Some(ArticleStatus::parse(raw).ok_or_else(|| {
            ApiError(AppError::Validation(format!("неизвестный статус «{raw}»")))
        })?),
        None => None,
    };

    let items = state.articles.list(status, q.limit, q.offset).await?;
    let total = state.articles.count(status).await?;

    Ok(Json(serde_json::json!({
        "items": items,
        "total": total,
        "counts": {
            "draft": state.articles.count(Some(ArticleStatus::Draft)).await?,
            "published": state.articles.count(Some(ArticleStatus::Published)).await?,
            "archived": state.articles.count(Some(ArticleStatus::Archived)).await?,
        },
        "pending_events": crate::storage::article::pending_event_count(&state.db).await?,
    })))
}

/// Одна статья целиком (для формы редактирования).
pub async fn admin_article(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<crate::services::article::Article>, ApiError> {
    check_admin(&headers, &state.config.admin.token)?;
    Ok(Json(state.articles.get(id).await?))
}

/// Создание статьи.
pub async fn admin_create_article(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ArticleInput>,
) -> Result<Json<crate::services::article::Article>, ApiError> {
    check_admin(&headers, &state.config.admin.token)?;
    Ok(Json(state.articles.create(input).await?))
}

/// Полное обновление статьи.
pub async fn admin_update_article(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(input): Json<ArticleInput>,
) -> Result<Json<crate::services::article::Article>, ApiError> {
    check_admin(&headers, &state.config.admin.token)?;
    Ok(Json(state.articles.update(id, input).await?))
}

/// Смена только статуса (быстрые кнопки «Опубликовать» / «Снять»).
#[derive(Deserialize)]
pub struct ArticleStatusUpdate {
    pub status: String,
}

pub async fn admin_update_article_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(body): Json<ArticleStatusUpdate>,
) -> Result<Json<crate::services::article::Article>, ApiError> {
    check_admin(&headers, &state.config.admin.token)?;

    let status = ArticleStatus::parse(&body.status).ok_or_else(|| {
        ApiError(AppError::Validation(format!(
            "неизвестный статус «{}»",
            body.status
        )))
    })?;

    Ok(Json(state.articles.set_status(id, status).await?))
}

/// Удаление статьи.
pub async fn admin_delete_article(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_admin(&headers, &state.config.admin.token)?;
    let removed = state.articles.delete(id).await?;
    Ok(Json(serde_json::json!({
        "success": true,
        "slug": removed.slug,
    })))
}

/// Предпросмотр markdown.
///
/// Рендерит тот же [`crate::utils::markdown::render`], что и страница статьи,
/// поэтому предпросмотр не может разойтись с публикацией.
#[derive(Deserialize)]
pub struct PreviewRequest {
    #[serde(default)]
    pub body_markdown: String,
}

pub async fn admin_preview_markdown(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PreviewRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_admin(&headers, &state.config.admin.token)?;

    if body.body_markdown.chars().count() > BODY_MAX_CHARS {
        return Err(AppError::Validation(format!(
            "текст длиннее {BODY_MAX_CHARS} символов"
        ))
        .into());
    }

    Ok(Json(serde_json::json!({
        "html": crate::utils::markdown::render(&body.body_markdown),
        "reading_time_minutes": crate::utils::markdown::reading_time_minutes(&body.body_markdown),
    })))
}

/// Журнал событий кросспостинга — для админки (видно, что заберёт n8n).
#[derive(Deserialize)]
pub struct OutboxQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
}

pub async fn admin_outbox(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<OutboxQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_admin(&headers, &state.config.admin.token)?;

    let status = q.status.as_deref().unwrap_or("pending");
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let events = crate::storage::article::list_events(&state.db, status, limit).await?;

    Ok(Json(serde_json::json!({
        "items": events.iter().map(event_json).collect::<Vec<_>>(),
        "pending": crate::storage::article::pending_event_count(&state.db).await?,
    })))
}

// ---------------------------------------------------------------------------
// Интеграция с n8n: лента событий (transactional outbox)
// ---------------------------------------------------------------------------

/// Проверяет ключ доступа интеграции.
///
/// Пока ключ не задан, фид выключен целиком и отвечает 404 — как и вебхук бота
/// настроек: нечего подтверждать существование незащищённого эндпоинта.
fn check_integration(headers: &HeaderMap, config: &crate::config::IntegrationsConfig) -> Result<(), ApiError> {
    if !config.is_active() {
        return Err(AppError::NotFound("integration feed disabled".into()).into());
    }

    let presented = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if presented == config.n8n_api_key {
        Ok(())
    } else {
        tracing::warn!("n8n feed: отклонён запрос с неверным ключом");
        Err(AppError::Unauthorized.into())
    }
}

/// `GET /api/integrations/outbox` — события для кросспостинга.
///
/// Читатель (n8n) забирает `pending`-события, публикует их по каналам и
/// подтверждает через `/ack`. Событие отдаётся снимком статьи, поэтому для
/// кросспостинга не нужен второй запрос за текстом.
pub async fn outbox_feed(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<OutboxQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_integration(&headers, &state.config.integrations)?;

    let status = q.status.as_deref().unwrap_or("pending");
    if !matches!(status, "pending" | "delivered" | "failed") {
        return Err(AppError::Validation(format!("неизвестный статус «{status}»")).into());
    }

    let limit = q
        .limit
        .unwrap_or(state.config.integrations.outbox_batch_limit)
        .clamp(1, 500);

    let events = crate::storage::article::list_events(&state.db, status, limit).await?;

    Ok(Json(serde_json::json!({
        "events": events.iter().map(event_json).collect::<Vec<_>>(),
        "count": events.len(),
        "base_url": state.articles.public_url(),
    })))
}

#[derive(Deserialize)]
pub struct OutboxAck {
    pub ids: Vec<i64>,
}

/// `POST /api/integrations/outbox/ack` — подтверждение доставки.
///
/// Идемпотентен: n8n может переподтвердить весь батч после сетевого сбоя, и
/// повторный вызов ничего не сломает.
pub async fn outbox_ack(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<OutboxAck>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_integration(&headers, &state.config.integrations)?;

    if body.ids.len() > 500 {
        return Err(AppError::Validation("слишком много id в одном запросе".into()).into());
    }

    let acked = crate::storage::article::ack_events(&state.db, &body.ids).await?;
    Ok(Json(serde_json::json!({ "acked": acked })))
}

#[derive(Deserialize)]
pub struct OutboxFail {
    pub id: i64,
    #[serde(default)]
    pub error: String,
}

/// `POST /api/integrations/outbox/fail` — сообщить о неудачной доставке.
///
/// После [`crate::storage::article::MAX_EVENT_ATTEMPTS`] попыток событие
/// уходит в `failed` и перестаёт попадаться в ленте: одно «отравленное»
/// событие не должно вечно ломать пайплайн.
pub async fn outbox_fail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<OutboxFail>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_integration(&headers, &state.config.integrations)?;

    let error: String = body.error.chars().take(500).collect();
    crate::storage::article::fail_event(&state.db, body.id, &error).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// Представление события для API: payload разворачивается в JSON, а не
/// отдаётся строкой — читателю не нужно парсить дважды.
fn event_json(event: &crate::storage::article::ArticleEvent) -> serde_json::Value {
    let payload: serde_json::Value = serde_json::from_str(&event.payload)
        .unwrap_or_else(|_| serde_json::json!({ "raw": event.payload }));

    serde_json::json!({
        "id": event.id,
        "article_id": event.article_id,
        "slug": event.article_slug,
        "event_type": event.event_type,
        "status": event.status,
        "attempts": event.attempts,
        "created_at": event.created_at,
        "delivered_at": event.delivered_at,
        "last_error": event.last_error,
        "payload": payload,
    })
}

// ---------------------------------------------------------------------------
// Static assets
// ---------------------------------------------------------------------------

pub async fn style_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        assets::STYLE_CSS,
    )
}

pub async fn main_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript")],
        assets::MAIN_JS,
    )
}

/// robots.txt. Собирается на лету из публичного адреса сайта: адрес карты в
/// директиве `Sitemap` обязан совпадать с тем, что отдаёт `/sitemap.xml`.
pub async fn robots(State(state): State<AppState>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        crate::services::seo::robots_txt(state.articles.public_url()),
    )
}

pub async fn manifest() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/manifest+json")],
        assets::MANIFEST_JSON,
    )
}

/// Карта сайта. Строится на лету: статические страницы + опубликованные статьи.
pub async fn sitemap(State(state): State<AppState>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        crate::services::feed::current_sitemap(state.articles.public_url()),
    )
}

/// RSS-лента блога — канал для n8n RSS Feed Trigger.
pub async fn rss(State(state): State<AppState>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        crate::services::feed::current_rss(state.articles.public_url()),
    )
}

pub async fn favicon() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/svg+xml")],
        assets::FAVICON_SVG,
    )
}

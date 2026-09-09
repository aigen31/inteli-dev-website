//! Обработчики HTTP API.

use axum::extract::{Path, Query, State};
use axum::http::header::{self, HeaderMap};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::api::{assets, client_ip, ApiError};
use crate::error::AppError;
use crate::services::chat::{ChatRequest, ChatResponse};
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

    if !state.rate_limiter.check(&ip_hash) {
        return Err(AppError::RateLimitExceeded { retry_after: 60 }.into());
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

pub async fn robots() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        assets::ROBOTS_TXT,
    )
}

pub async fn manifest() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/manifest+json")],
        assets::MANIFEST_JSON,
    )
}

pub async fn sitemap() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        assets::SITEMAP_XML,
    )
}

pub async fn favicon() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/svg+xml")],
        assets::FAVICON_SVG,
    )
}

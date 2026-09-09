//! HTTP API Layer (axum/hyper).
//!
//! JSON-эндпоинты для чата, заявок, статуса и админки. SSR-страницы монтирует
//! `leptos_axum` в `main.rs`. Статика (CSS/JS/robots/manifest) отдаётся из
//! встроенных строк (`assets.rs`), чтобы образ оставался самодостаточным.

pub mod assets;
pub mod handlers;

use axum::http::header::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};

use crate::error::AppError;
use crate::state::AppState;

/// Регистрирует все API-маршруты (без `.with_state` — его задаёт `main`).
pub fn routes() -> Router<AppState> {
    Router::new()
        // Public
        .route("/api/health", get(handlers::health))
        .route("/api/status", get(handlers::status))
        .route("/api/status.json", get(handlers::status))
        .route("/api/chat", post(handlers::chat))
        .route("/api/lead", post(handlers::lead))
        .route("/api/telegram/webhook", post(handlers::telegram_webhook))
        // Admin (token auth)
        .route("/api/admin/stats", get(handlers::admin_stats))
        .route("/api/admin/leads", get(handlers::admin_leads))
        .route("/api/admin/chats", get(handlers::admin_chats))
        .route("/api/admin/leads/{id}", patch(handlers::admin_update_lead))
        // Static assets (embedded)
        .route("/assets/style.css", get(handlers::style_css))
        .route("/assets/main.js", get(handlers::main_js))
        .route("/robots.txt", get(handlers::robots))
        .route("/manifest.json", get(handlers::manifest))
        .route("/sitemap.xml", get(handlers::sitemap))
        .route("/favicon.svg", get(handlers::favicon))
}

/// Обёртка `AppError` для конвертации в HTTP-ответ.
pub struct ApiError(pub AppError);

impl From<AppError> for ApiError {
    fn from(e: AppError) -> Self {
        Self(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.0.http_status();
        let body = Json(serde_json::json!({ "error": self.0.to_string() }));
        (status, body).into_response()
    }
}

/// Извлекает IP клиента из заголовков (за прокси) с fallback.
pub fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unknown".to_string())
}

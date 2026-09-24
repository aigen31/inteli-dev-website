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
        // Бот настроек сайта: Telegram стучится сюда (секрет — в заголовке).
        .route(
            "/api/settings-bot/webhook",
            post(handlers::settings_bot_webhook),
        )
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
        let body = match &self.0 {
            // Лимит — это не сбой, а часть UX: отдаём человеческий текст и
            // сколько секунд ждать, чтобы клиент мог показать понятный отсчёт.
            AppError::RateLimitExceeded { retry_after } => serde_json::json!({
                "error": rate_limit_message(*retry_after),
                "retry_after": retry_after,
            }),
            // Тексты валидации уже написаны для пользователя — не портим их
            // техническим префиксом «validation error:».
            AppError::Validation(msg) => serde_json::json!({ "error": msg }),
            other => serde_json::json!({ "error": other.to_string() }),
        };
        (status, Json(body)).into_response()
    }
}

/// Текст для пользователя при исчерпании квоты.
fn rate_limit_message(retry_after_secs: u64) -> String {
    let minutes = retry_after_secs.div_ceil(60).max(1);
    if minutes >= 60 {
        return "Лимит AI-ответов исчерпан. Попробуйте снова через час — или напишите мне в Telegram.".to_string();
    }
    format!(
        "Слишком много запросов. Попробуйте снова через {} {}.",
        minutes,
        crate::limits::plural_ru(minutes as u32, "минуту", "минуты", "минут")
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_message_pluralizes_minutes() {
        assert!(rate_limit_message(60).contains("через 1 минуту"));
        assert!(rate_limit_message(300).contains("через 5 минут"));
        assert!(rate_limit_message(2413).contains("через 41 минуту"));
    }

    #[test]
    fn rate_limit_message_switches_to_hours_at_the_boundary() {
        // 3599 с — это уже «через час», а не «через 60 минут».
        assert!(rate_limit_message(3599).contains("через час"));
        assert!(rate_limit_message(3600).contains("через час"));
    }

    #[test]
    fn client_ip_prefers_forwarded_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.7, 10.0.0.1".parse().unwrap());
        assert_eq!(client_ip(&headers), "203.0.113.7");
    }
}

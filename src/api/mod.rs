//! HTTP API Layer (axum/hyper).
//!
//! JSON-эндпоинты для чата, заявок, статуса и админки. SSR-страницы монтирует
//! `leptos_axum` в `main.rs`. Статика (CSS/JS/robots/manifest) отдаётся из
//! встроенных строк (`assets.rs`), чтобы образ оставался самодостаточным.

pub mod assets;
pub mod handlers;

use axum::http::header::HeaderMap;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};

use crate::config::SeoConfig;
use crate::error::AppError;
use crate::state::AppState;

/// Файлы в корне сайта, которые отдаёт само приложение.
///
/// Список существует ради проверки конфигурации: имя из `[seo].root_files` не
/// должно совпасть с этими путями, иначе matchit паникует на конфликте маршрутов
/// и приложение не поднимется. Он же проверяется тестом, который дёргает каждый
/// путь и ждёт 200, — так список не может разойтись с реальными маршрутами.
pub const ROOT_ASSET_PATHS: &[&str] = &[
    "/robots.txt",
    "/manifest.json",
    "/sitemap.xml",
    "/rss.xml",
    "/feed.xml",
    "/favicon.svg",
];

/// Регистрирует все API-маршруты (без `.with_state` — его задаёт `main`).
///
/// `seo` нужен для файлов в корне: подтверждение прав и ключ IndexNow — это
/// обычные текстовые файлы, но их имена известны только из конфигурации.
pub fn routes(seo: &SeoConfig) -> Router<AppState> {
    let router = Router::new()
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
        // ВАЖНО: axum 0.7 (matchit 0.7) использует синтаксис `:id`. Запись
        // `{id}` появилась только в axum 0.8 и здесь матчится как литеральный
        // сегмент — маршрут молча превращается в 404.
        .route("/api/admin/stats", get(handlers::admin_stats))
        .route("/api/admin/leads", get(handlers::admin_leads))
        .route("/api/admin/chats", get(handlers::admin_chats))
        .route("/api/admin/leads/:id", patch(handlers::admin_update_lead))
        // Статьи блога (админка).
        .route(
            "/api/admin/articles",
            get(handlers::admin_articles).post(handlers::admin_create_article),
        )
        // Предпросмотр markdown лежит ВНЕ `/articles/…`, чтобы статический
        // сегмент не спорил с параметрическим маршрутом статьи.
        .route(
            "/api/admin/article-preview",
            post(handlers::admin_preview_markdown),
        )
        .route(
            "/api/admin/articles/:id",
            get(handlers::admin_article)
                .put(handlers::admin_update_article)
                .patch(handlers::admin_update_article_status)
                .delete(handlers::admin_delete_article),
        )
        .route("/api/admin/outbox", get(handlers::admin_outbox))
        // Кросспостинг: фид событий для n8n (ключ в заголовке `X-Api-Key`).
        .route("/api/integrations/outbox", get(handlers::outbox_feed))
        .route("/api/integrations/outbox/ack", post(handlers::outbox_ack))
        .route("/api/integrations/outbox/fail", post(handlers::outbox_fail))
        // Static assets (embedded)
        .route("/assets/style.css", get(handlers::style_css))
        .route("/assets/main.js", get(handlers::main_js))
        .route("/robots.txt", get(handlers::robots))
        .route("/manifest.json", get(handlers::manifest))
        .route("/sitemap.xml", get(handlers::sitemap))
        .route("/rss.xml", get(handlers::rss))
        .route("/feed.xml", get(handlers::rss))
        .route("/favicon.svg", get(handlers::favicon));

    with_root_files(router, seo)
}

/// Регистрирует файлы в корне из `[seo]`: подтверждение прав и ключ IndexNow.
///
/// Имена приходят из конфигурации, поэтому [`SeoConfig::validate`] обязан
/// отработать до этого места: столкновение с уже зарегистрированным путём
/// (`ROOT_ASSET_PATHS`) — это паника в недрах matchit, а не понятная ошибка.
fn with_root_files(mut router: Router<AppState>, seo: &SeoConfig) -> Router<AppState> {
    for (name, content) in seo.root_files() {
        let content_type = crate::services::seo::content_type_for(&name);
        let path = format!("/{name}");

        router = router.route(
            &path,
            get(move || {
                let body = content.clone();
                async move { ([(CONTENT_TYPE, content_type)], body) }
            }),
        );
    }

    router
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

    #[test]
    fn root_asset_paths_are_all_reserved() {
        // Список в `services::seo` защищает от конфигурации, которая столкнёт
        // маршруты. Если здесь появится путь, которого там нет, — приложение
        // упадёт на старте, а не отдаст понятную ошибку конфига.
        for path in ROOT_ASSET_PATHS {
            let name = path.trim_start_matches('/');
            assert!(
                crate::services::seo::RESERVED_ROOT_NAMES.contains(&name),
                "`{path}` отдаётся статикой, но его имени нет в RESERVED_ROOT_NAMES"
            );
        }
    }
}

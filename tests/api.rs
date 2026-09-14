//! Интеграционные тесты HTTP API (axum Router + реальный SQLite).
//!
//! Проверяют слой API end-to-end: health, status, chat, lead, admin-авторизацию.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::header::{self, HeaderValue};
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use inteli_dev::api;
use inteli_dev::cache::{InMemoryCache, RateLimiter, SlidingWindowLimiter};
use inteli_dev::config::AppConfig;
use inteli_dev::error::AppResult;
use inteli_dev::llm::{ChatProvider, Message};
use inteli_dev::memory::content::{fallback_content, SiteContent};
use inteli_dev::memory::OpenVikingClient;
use inteli_dev::notification::NotificationService;
use inteli_dev::services::{ChatService, LeadService};
use inteli_dev::state::AppState;

const ADMIN_TOKEN: &str = "test-admin-token";

/// Заглушка LLM-провайдера (не делает реальных запросов).
struct MockProvider;

#[async_trait::async_trait]
impl ChatProvider for MockProvider {
    async fn chat_with_history(&self, _messages: Vec<Message>) -> AppResult<String> {
        Ok("MOCK ANSWER".to_string())
    }
}

fn test_config() -> AppConfig {
    toml::from_str(
        r#"
[server]
host = "127.0.0.1"
port = 8080

[database]
path = "test.db"

[cache]
enabled = true
ttl_seconds = 3600

[openviking]
base_url = "http://127.0.0.1:1"

[llm]
provider = "deepseek"
base_url = "http://x"
model = "m"

[telegram]
bot_token = ""
admin_chat_id = 0

[vk]
oauth_token = ""
admin_user_id = 0

[admin]
token = "test-admin-token"

[ratelimit]
max_requests_per_minute = 1000
window_seconds = 60

[logging]
level = "info"

[security]
ip_hash_algorithm = "sha256"
"#,
    )
    .expect("valid test config")
}

async fn test_state() -> AppState {
    test_state_with_quota(1000).await
}

/// Состояние с настраиваемой часовой квотой на AI-ответы.
async fn test_state_with_quota(llm_requests_per_hour: u32) -> AppState {
    SiteContent::set_global(fallback_content());

    let config = Arc::new(test_config());
    let db_path = std::env::temp_dir().join(format!("inteli_test_{}.db", uuid::Uuid::new_v4()));
    let db = inteli_dev::storage::init_pool(db_path.to_str().expect("path"))
        .await
        .expect("db");

    let llm: Arc<dyn ChatProvider> = Arc::new(MockProvider);
    let memory = Arc::new(OpenVikingClient::new(
        "http://127.0.0.1:1",
        Duration::from_millis(10),
    ));
    let cache = Arc::new(InMemoryCache::new(Duration::from_secs(3600)));
    let rate_limiter = Arc::new(RateLimiter::new(1000, Duration::from_secs(60)));
    // Часовая квота: в большинстве сценариев щедрая, чтобы обычные запросы не
    // упирались в лимит; точечно проверяется в тестах на квоту.
    let hourly_limiter = Arc::new(SlidingWindowLimiter::new(
        llm_requests_per_hour,
        Duration::from_secs(3600),
    ));
    let notifications = Arc::new(NotificationService::new(0, String::new(), String::new(), 0));
    let chat = Arc::new(ChatService::new(llm, memory, cache.clone(), db.clone()));
    let leads = Arc::new(LeadService::new(db.clone(), notifications.clone()));
    let leptos_options = leptos::prelude::get_configuration(None)
        .map(|c| c.leptos_options)
        .expect("leptos options");

    AppState {
        config,
        db,
        chat,
        leads,
        notifications,
        cache,
        rate_limiter,
        hourly_limiter,
        leptos_options,
        started_at: Instant::now(),
    }
}

/// Отправляет запрос к API и возвращает (статус, JSON-тело).
async fn request(
    state: &AppState,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
    token: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let app = api::routes().with_state(state.clone());

    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        builder = builder.header(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {t}")).unwrap(),
        );
    }

    let req = match body {
        Some(b) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };

    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

#[tokio::test]
async fn health_returns_healthy() {
    let state = test_state().await;
    let (status, body) = request(&state, "GET", "/api/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "healthy");
}

#[tokio::test]
async fn status_returns_availability() {
    let state = test_state().await;
    let (status, body) = request(&state, "GET", "/api/status", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "available");
}

#[tokio::test]
async fn chat_preset_answers_without_llm() {
    let state = test_state().await;
    let body =
        serde_json::json!({ "message": "кто вы", "question_type": "preset", "preset_index": 0 });
    let (status, resp) = request(&state, "POST", "/api/chat", Some(body), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resp["source"], "openviking_direct");

    // Имя берём из контента, а не хардкодом: прежний литерал «Иван Петров»
    // остался от SEO-версии сайта и ломал тест после смены позиционирования.
    let expected = fallback_content().profile.name;
    assert!(
        resp["answer"].as_str().unwrap_or("").contains(&expected),
        "ответ должен содержать имя {expected}, получено: {resp}"
    );
}

#[tokio::test]
async fn lead_creates_and_returns_id() {
    let state = test_state().await;
    let body = serde_json::json!({
        "name": "Иван",
        "email": "i@example.com",
        "message": "нужен SEO",
        "source": "form"
    });
    let (status, resp) = request(&state, "POST", "/api/lead", Some(body), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resp["success"], true);
    assert!(resp["lead_id"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn lead_rejects_invalid_email() {
    let state = test_state().await;
    let body = serde_json::json!({ "name": "Иван", "email": "not-an-email", "message": "x", "source": "form" });
    let (status, _) = request(&state, "POST", "/api/lead", Some(body), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn lead_rejects_overly_long_message() {
    let state = test_state().await;
    let max = inteli_dev::limits::Limits::get().lead_message_max_chars;
    let body = serde_json::json!({
        "name": "Иван",
        "message": "а".repeat(max + 1),
        "source": "form"
    });
    let (status, resp) = request(&state, "POST", "/api/lead", Some(body), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        resp["error"].as_str().unwrap_or("").contains("длиннее"),
        "получено: {resp}"
    );
}

#[tokio::test]
async fn lead_rejects_overly_long_name() {
    let state = test_state().await;
    let max = inteli_dev::limits::Limits::get().lead_name_max_chars;
    let body = serde_json::json!({
        "name": "а".repeat(max + 1),
        "message": "нужен SEO",
        "source": "form"
    });
    let (status, _) = request(&state, "POST", "/api/lead", Some(body), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn chat_rejects_empty_message() {
    let state = test_state().await;
    let body = serde_json::json!({ "message": "   ", "question_type": "free" });
    let (status, _) = request(&state, "POST", "/api/chat", Some(body), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn chat_rejects_overly_long_message() {
    let state = test_state().await;
    let max = inteli_dev::limits::Limits::get().chat_message_max_chars;
    let body = serde_json::json!({
        "message": "а".repeat(max + 1),
        "question_type": "free"
    });
    let (status, resp) = request(&state, "POST", "/api/chat", Some(body), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        resp["error"].as_str().unwrap_or("").contains("длинн"),
        "получено: {resp}"
    );
}

#[tokio::test]
async fn chat_accepts_message_at_the_limit() {
    let state = test_state().await;
    let max = inteli_dev::limits::Limits::get().chat_message_max_chars;
    let body = serde_json::json!({
        "message": "а".repeat(max),
        "question_type": "free"
    });
    let (status, _) = request(&state, "POST", "/api/chat", Some(body), None).await;
    assert_eq!(status, StatusCode::OK, "ровно лимит должен приниматься");
}

#[tokio::test]
async fn hourly_quota_blocks_fourth_ai_request() {
    let state = test_state_with_quota(3).await;

    for i in 0..3 {
        // Разные вопросы: одинаковые попали бы в кэш и не тратили квоту.
        let body = serde_json::json!({
            "message": format!("вопрос номер {i}"),
            "question_type": "free"
        });
        let (status, _) = request(&state, "POST", "/api/chat", Some(body), None).await;
        assert_eq!(status, StatusCode::OK, "запрос {i} должен пройти");
    }

    let body = serde_json::json!({ "message": "четвёртый вопрос", "question_type": "free" });
    let (status, resp) = request(&state, "POST", "/api/chat", Some(body), None).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);

    // Пользователь должен увидеть понятный текст и время ожидания.
    let err = resp["error"].as_str().unwrap_or("");
    assert!(
        err.contains("Лимит") || err.contains("Слишком много"),
        "технический текст вместо человеческого: {err}"
    );
    assert!(resp["retry_after"].as_u64().unwrap_or(0) > 0);
}

#[tokio::test]
async fn presets_do_not_consume_ai_quota() {
    let state = test_state_with_quota(1).await;

    // Единственный AI-ответ за час.
    let body = serde_json::json!({ "message": "свободный вопрос", "question_type": "free" });
    let (status, _) = request(&state, "POST", "/api/chat", Some(body), None).await;
    assert_eq!(status, StatusCode::OK);

    // Квота исчерпана, но preset/availability/lead_request идут из контента.
    for (kind, index) in [("preset", 0), ("preset", 1), ("availability", 4), ("lead_request", 5)] {
        let body = serde_json::json!({
            "message": "preset",
            "question_type": kind,
            "preset_index": index
        });
        let (status, resp) = request(&state, "POST", "/api/chat", Some(body), None).await;
        assert_eq!(status, StatusCode::OK, "{kind}/{index} не должен упираться в квоту LLM");
        assert_eq!(resp["source"], "openviking_direct");
    }
}

#[tokio::test]
async fn cached_answer_does_not_consume_ai_quota() {
    let state = test_state_with_quota(1).await;

    let body = serde_json::json!({ "message": "Сколько стоит?", "question_type": "free" });
    let (s1, r1) = request(&state, "POST", "/api/chat", Some(body), None).await;
    assert_eq!(s1, StatusCode::OK);
    assert_eq!(r1["source"], "llm");

    // Тот же вопрос другим регистром — из кэша, квота не тратится.
    let body2 = serde_json::json!({ "message": "сколько СТОИТ?", "question_type": "free" });
    let (s2, r2) = request(&state, "POST", "/api/chat", Some(body2), None).await;
    assert_eq!(s2, StatusCode::OK, "повтор из кэша не должен упираться в квоту");
    assert_eq!(r2["source"], "cached");
}

#[tokio::test]
async fn admin_requires_token() {
    let state = test_state().await;
    let (status, _) = request(&state, "GET", "/api/admin/stats", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_with_token_returns_stats() {
    let state = test_state().await;
    let (status, body) = request(&state, "GET", "/api/admin/stats", None, Some(ADMIN_TOKEN)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("leads_today").is_some());
    assert!(body.get("chats_today").is_some());
}

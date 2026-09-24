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
use inteli_dev::config::{AppConfig, SettingsBotConfig};
use inteli_dev::error::AppResult;
use inteli_dev::llm::{ChatProvider, Message};
use inteli_dev::memory::content::{fallback_content, SiteContent};
use inteli_dev::memory::OpenVikingClient;
use inteli_dev::notification::NotificationService;
use inteli_dev::services::{ArticleService, ChatService, LeadService, SettingsBot};
use inteli_dev::state::AppState;

const ADMIN_TOKEN: &str = "test-admin-token";
/// Секрет вебхука бота настроек в тестах (см. `test_settings_bot_config`).
const BOT_SECRET: &str = "test-webhook-secret";
/// Telegram id владельца в тестах.
const BOT_ADMIN_ID: i64 = 330711327;

/// Доступ к живому состоянию сайта для тестов.
///
/// [`SiteSettings`] — глобальный `RwLock`, а тесты одного бинарника идут
/// параллельно и делят процесс: статус, выставленный одним тестом, иначе
/// утекал бы в другой. Поэтому все тесты, которые читают или меняют статус,
/// берут этот замок, а вместе с ним — чистую исходную публикацию.
fn availability_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    // Отравление возможно только при панике внутри самих тестов — не повод
    // ронять остальные.
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    inteli_dev::settings::SiteSettings::reset_global();
    inteli_dev::settings::SiteSettings::set_global(inteli_dev::settings::defaults_from_content());
    guard
}

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

[integrations]
n8n_api_key = "test-n8n-key"
outbox_batch_limit = 20
"#,
    )
    .expect("valid test config")
}

async fn test_state() -> AppState {
    test_state_with_quota(1000).await
}

/// Состояние с настраиваемой часовой квотой на AI-ответы.
async fn test_state_with_quota(llm_requests_per_hour: u32) -> AppState {
    test_state_with(test_settings_bot_config(), llm_requests_per_hour).await
}

/// Состояние с произвольной конфигурацией бота настроек.
///
/// `api_base` указывает на закрытый локальный порт: бот не должен ходить в
/// реальный Telegram из тестов, а соединение там отбивается мгновенно.
async fn test_state_with(bot_config: SettingsBotConfig, llm_requests_per_hour: u32) -> AppState {
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
    let settings_bot = Arc::new(SettingsBot::new(&bot_config));
    let articles = Arc::new(ArticleService::new(db.clone(), test_public_url()));
    let leptos_options = leptos::prelude::get_configuration(None)
        .map(|c| c.leptos_options)
        .expect("leptos options");

    AppState {
        config,
        db,
        chat,
        leads,
        articles,
        notifications,
        cache,
        rate_limiter,
        hourly_limiter,
        settings_bot,
        leptos_options,
        started_at: Instant::now(),
    }
}

/// Публичный адрес сайта в тестах — из него собираются ссылки в RSS и событиях.
fn test_public_url() -> &'static str {
    "https://test.inteli-dev.ru"
}

/// Конфигурация бота настроек для тестов: владелец — id 330711327.
fn test_settings_bot_config() -> SettingsBotConfig {
    SettingsBotConfig {
        token: "test-token".into(),
        admin_id: BOT_ADMIN_ID,
        webhook_secret: BOT_SECRET.into(),
        api_base: "http://127.0.0.1:1".into(),
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
    let _guard = availability_lock();
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
async fn admin_can_update_lead_status() {
    let state = test_state().await;

    let (_, lead) = request(
        &state,
        "POST",
        "/api/lead",
        Some(serde_json::json!({
            "name": "Иван",
            "email": "i@example.com",
            "message": "нужен проект",
            "source": "form"
        })),
        None,
    )
    .await;
    let id = lead["lead_id"].as_i64().expect("id заявки");

    // Регрессия: маршрут был объявлен как `/api/admin/leads/{id}`, а axum 0.7
    // понимает только `:id`. `{id}` матчился как литеральный сегмент, поэтому
    // PATCH отвечал 404 и статус заявки нельзя было сменить из админки.
    let (status, _) = request(
        &state,
        "PATCH",
        &format!("/api/admin/leads/{id}"),
        Some(serde_json::json!({ "status": "contacted" })),
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, leads) = request(
        &state,
        "GET",
        "/api/admin/leads?limit=10",
        None,
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(leads[0]["status"], "contacted");
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

    // Квота исчерпана, но все кнопки без ИИ идут из контента. Берём список из
    // общего источника правды: добавили кнопку — она сразу проверяется здесь.
    for preset in inteli_dev::services::chat::PRESETS
        .iter()
        .filter(|p| p.kind != "analysis")
    {
        let body = serde_json::json!({
            "message": preset.label,
            "question_type": preset.kind,
            "preset_index": preset.index
        });
        let (status, resp) = request(&state, "POST", "/api/chat", Some(body), None).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "«{}» ({}/{}) упёрлась в квоту LLM",
            preset.label,
            preset.kind,
            preset.index
        );
        assert_eq!(
            resp["source"], "openviking_direct",
            "«{}» должна отвечать из контента, а не через модель",
            preset.label
        );
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

// ---------------------------------------------------------------------------
// Бот настроек сайта
// ---------------------------------------------------------------------------

/// Отправляет апдейт Telegram на вебхук бота настроек.
async fn webhook(state: &AppState, secret: Option<&str>, update: serde_json::Value) -> StatusCode {
    let app = api::routes().with_state(state.clone());

    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/settings-bot/webhook")
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(secret) = secret {
        builder = builder.header(
            inteli_dev::services::settings_bot::SECRET_HEADER,
            HeaderValue::from_str(secret).unwrap(),
        );
    }

    app.oneshot(builder.body(Body::from(update.to_string())).unwrap())
        .await
        .unwrap()
        .status()
}

/// Апдейт-сообщение от указанного пользователя.
fn message_update(user_id: i64, text: &str) -> serde_json::Value {
    serde_json::json!({
        "update_id": 1,
        "message": {
            "message_id": 10,
            "from": { "id": user_id, "first_name": "Test", "is_bot": false },
            "chat": { "id": user_id, "type": "private" },
            "date": 1780000000,
            "text": text
        }
    })
}

#[tokio::test]
async fn settings_bot_webhook_rejects_wrong_or_missing_secret() {
    let _guard = availability_lock();
    let state = test_state().await;

    let wrong = webhook(&state, Some("не-тот-секрет"), message_update(BOT_ADMIN_ID, "/busy")).await;
    assert_eq!(wrong, StatusCode::NOT_FOUND);

    let missing = webhook(&state, None, message_update(BOT_ADMIN_ID, "/busy")).await;
    assert_eq!(missing, StatusCode::NOT_FOUND);

    // Статус не изменился ни в одном случае.
    let (_, body) = request(&state, "GET", "/api/status", None, None).await;
    assert_eq!(body["status"], "available");
}

#[tokio::test]
async fn settings_bot_webhook_is_404_when_not_configured() {
    let _guard = availability_lock();
    // Нет секрета → бот не активен, эндпоинт не должен существовать.
    let inactive = SettingsBotConfig {
        token: String::new(),
        admin_id: 0,
        webhook_secret: String::new(),
        api_base: "http://127.0.0.1:1".into(),
    };
    let state = test_state_with(inactive, 1000).await;

    let status = webhook(&state, Some(BOT_SECRET), message_update(BOT_ADMIN_ID, "/busy")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn settings_bot_ignores_foreign_users() {
    let _guard = availability_lock();
    let state = test_state().await;

    // Не владелец: получает 200 (Telegram не должен ретраить) и отказ в ответе.
    let status = webhook(&state, Some(BOT_SECRET), message_update(999_999, "/full")).await;
    assert_eq!(status, StatusCode::OK);

    let (_, body) = request(&state, "GET", "/api/status", None, None).await;
    assert_eq!(body["status"], "available", "чужой не должен менять статус");
}

#[tokio::test]
async fn settings_bot_changes_status_end_to_end() {
    let _guard = availability_lock();
    let state = test_state().await;

    let status = webhook(&state, Some(BOT_SECRET), message_update(BOT_ADMIN_ID, "/busy")).await;
    assert_eq!(status, StatusCode::OK);

    // 1. Изменился публичный API статуса.
    let (_, body) = request(&state, "GET", "/api/status", None, None).await;
    assert_eq!(body["status"], "busy");
    assert_eq!(body["label"], "Приём заявок: ограничен");
    assert_eq!(body["icon_name"], "settings");

    // 2. Значение ушло в БД — переживёт перезапуск.
    let saved = inteli_dev::settings::load_or_default(&state.db).await.unwrap();
    assert_eq!(saved.availability.status, "busy");

    // 3. И сразу поменялось в памяти — это то, что читают SSR-страницы.
    assert_eq!(inteli_dev::settings::SiteSettings::availability().status, "busy");
}

#[tokio::test]
async fn settings_bot_updates_projects_and_slot() {
    let _guard = availability_lock();
    let state = test_state().await;

    webhook(&state, Some(BOT_SECRET), message_update(BOT_ADMIN_ID, "/projects 4")).await;
    webhook(
        &state,
        Some(BOT_SECRET),
        message_update(BOT_ADMIN_ID, "/slot со 2 ноября"),
    )
    .await;

    let (_, body) = request(&state, "GET", "/api/status", None, None).await;
    assert_eq!(body["current_projects"], 4);
    assert_eq!(body["next_free_slot"], "со 2 ноября");
}

#[tokio::test]
async fn settings_bot_rejects_invalid_values_without_changing_state() {
    let _guard = availability_lock();
    let state = test_state().await;

    for bad in ["/projects 500", "/projects abc", "/nonsense", "/slot"] {
        let status = webhook(&state, Some(BOT_SECRET), message_update(BOT_ADMIN_ID, bad)).await;
        assert_eq!(status, StatusCode::OK, "команда «{bad}» должна обрабатываться");
    }

    let (_, body) = request(&state, "GET", "/api/status", None, None).await;
    assert_eq!(body["status"], "available");
    assert_eq!(body["current_projects"], fallback_content().availability.current_projects);
}

#[tokio::test]
async fn settings_bot_handles_inline_button_callbacks() {
    let _guard = availability_lock();
    let state = test_state().await;

    let update = serde_json::json!({
        "update_id": 7,
        "callback_query": {
            "id": "cb-42",
            "from": { "id": BOT_ADMIN_ID, "is_bot": false },
            "message": {
                "message_id": 55,
                "chat": { "id": BOT_ADMIN_ID, "type": "private" }
            },
            "data": "set:full"
        }
    });

    let status = webhook(&state, Some(BOT_SECRET), update).await;
    assert_eq!(status, StatusCode::OK);

    let (_, body) = request(&state, "GET", "/api/status", None, None).await;
    assert_eq!(body["status"], "full");
}

#[tokio::test]
async fn settings_bot_state_is_visible_to_the_chatbot() {
    let _guard = availability_lock();
    let state = test_state().await;
    webhook(&state, Some(BOT_SECRET), message_update(BOT_ADMIN_ID, "/full")).await;

    // Ответ AI-чата про занятость берётся из того же живого состояния.
    let body = serde_json::json!({ "message": "Когда вы свободны?", "question_type": "availability" });
    let (status, resp) = request(&state, "POST", "/api/chat", Some(body), None).await;
    assert_eq!(status, StatusCode::OK);
    // Ожидание берём из словаря статуса, а не литералом: иначе тест закрепит
    // формулировку, которую бейдж, бот и API могут сменить согласованно.
    let expected = inteli_dev::services::status::status_line("full");
    assert!(
        resp["answer"].as_str().unwrap_or("").contains(&expected),
        "чат должен отдавать живой статус «{expected}», получено: {resp}"
    );
}

#[tokio::test]
async fn availability_question_is_persisted_to_history() {
    // Регрессия: CHECK(question_type) не знал значения 'availability', вставка
    // падала, а save_chat глотает ошибку — вопрос терялся молча.
    let state = test_state().await;

    let body = serde_json::json!({ "message": "Когда вы свободны?", "question_type": "availability" });
    let (status, _) = request(&state, "POST", "/api/chat", Some(body), None).await;
    assert_eq!(status, StatusCode::OK);

    let (_, chats) = request(&state, "GET", "/api/admin/chats?limit=10", None, Some(ADMIN_TOKEN)).await;
    let rows = chats.as_array().expect("список чатов");
    assert_eq!(rows.len(), 1, "вопрос о занятости должен сохраняться");
    assert_eq!(rows[0]["question_type"], "availability");
}

// ---------------------------------------------------------------------------
// Статьи блога и кросспостинг
// ---------------------------------------------------------------------------

/// Ключ фида событий в тестах (см. `[integrations]` в `test_config`).
const N8N_KEY: &str = "test-n8n-key";

/// Замок для тестов статей.
///
/// [`PublishedArticles`] — глобальный снимок в памяти процесса. Тесты одного
/// бинарника идут параллельно, поэтому без общего замка статья, опубликованная
/// одним тестом, попадала бы в RSS и sitemap другого.
fn articles_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    inteli_dev::services::PublishedArticles::reset_global();
    guard
}

/// Запрос с произвольными заголовками (нужен `X-Api-Key` для фида n8n).
async fn request_with_headers(
    state: &AppState,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
    headers: &[(&str, &str)],
) -> (StatusCode, serde_json::Value) {
    let app = api::routes().with_state(state.clone());

    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        builder = builder.header(*name, HeaderValue::from_str(value).unwrap());
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

/// Запрос, ответ которого — не JSON (RSS/sitemap отдают XML).
async fn request_text(state: &AppState, uri: &str) -> (StatusCode, String) {
    let app = api::routes().with_state(state.clone());
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// Создаёт статью через админку и возвращает её JSON.
async fn create_article(
    state: &AppState,
    title: &str,
    status: &str,
) -> serde_json::Value {
    let body = serde_json::json!({
        "title": title,
        "summary": "Краткое описание",
        "body_markdown": "# Заголовок\n\nТекст **статьи**.",
        "tags": ["rust", "ai"],
        "status": status,
    });

    let (code, article) = request(
        state,
        "POST",
        "/api/admin/articles",
        Some(body),
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "создание статьи: {article}");
    article
}

#[tokio::test]
async fn admin_articles_require_token() {
    let _guard = articles_lock();
    let state = test_state().await;

    let (status, _) = request(&state, "GET", "/api/admin/articles", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, body) = request(
        &state,
        "GET",
        "/api/admin/articles",
        None,
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 0);
}

#[tokio::test]
async fn article_crud_flow_end_to_end() {
    let _guard = articles_lock();
    let state = test_state().await;

    // 1. Создание черновика: slug выводится из заголовка транслитом.
    let created = create_article(&state, "Как я собирал AI-сервер", "draft").await;
    let id = created["id"].as_i64().expect("id статьи");
    assert_eq!(created["slug"], "kak-ya-sobiral-ai-server");
    assert_eq!(created["status"], "draft");
    assert!(created["published_at"].is_null());

    // 2. Чтение одной статьи.
    let (status, one) = request(
        &state,
        "GET",
        &format!("/api/admin/articles/{id}"),
        None,
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(one["title"], "Как я собирал AI-сервер");

    // 3. Обновление.
    let body = serde_json::json!({
        "title": "Как я собирал AI-сервер (обновлено)",
        "summary": "Новое описание",
        "body_markdown": "Текст v2",
        "tags": ["gpu"],
        "status": "draft",
    });
    let (status, updated) = request(
        &state,
        "PUT",
        &format!("/api/admin/articles/{id}"),
        Some(body),
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["title"], "Как я собирал AI-сервер (обновлено)");
    assert_eq!(updated["tags"][0], "gpu");

    // 4. Публикация через PATCH: дата появляется.
    let (status, published) = request(
        &state,
        "PATCH",
        &format!("/api/admin/articles/{id}"),
        Some(serde_json::json!({ "status": "published" })),
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(published["status"], "published");
    assert!(!published["published_at"].is_null());

    // 5. Снятие с публикации возвращает в черновики и обнуляет дату.
    let (_, drafted) = request(
        &state,
        "PATCH",
        &format!("/api/admin/articles/{id}"),
        Some(serde_json::json!({ "status": "draft" })),
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(drafted["status"], "draft");
    assert!(drafted["published_at"].is_null());

    // 6. Удаление.
    let (status, _) = request(
        &state,
        "DELETE",
        &format!("/api/admin/articles/{id}"),
        None,
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = request(
        &state,
        "GET",
        &format!("/api/admin/articles/{id}"),
        None,
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn article_validation_rejects_bad_input() {
    let _guard = articles_lock();
    let state = test_state().await;

    // Пустой заголовок.
    let (status, body) = request(
        &state,
        "POST",
        "/api/admin/articles",
        Some(serde_json::json!({ "title": "  ", "body_markdown": "текст" })),
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap_or("").contains("заголовок"));

    // Пустое тело.
    let (status, _) = request(
        &state,
        "POST",
        "/api/admin/articles",
        Some(serde_json::json!({ "title": "Заголовок", "body_markdown": "" })),
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Опасная схема в обложке.
    let (status, _) = request(
        &state,
        "POST",
        "/api/admin/articles",
        Some(serde_json::json!({
            "title": "Заголовок",
            "body_markdown": "текст",
            "cover_image_url": "javascript:alert(1)"
        })),
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Неизвестный статус в PATCH.
    let created = create_article(&state, "Черновик", "draft").await;
    let id = created["id"].as_i64().unwrap();
    let (status, _) = request(
        &state,
        "PATCH",
        &format!("/api/admin/articles/{id}"),
        Some(serde_json::json!({ "status": "bogus" })),
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn duplicate_slug_gets_a_free_suffix() {
    let _guard = articles_lock();
    let state = test_state().await;

    let first = create_article(&state, "Одна статья", "draft").await;
    let second = create_article(&state, "Одна статья", "draft").await;

    assert_eq!(first["slug"], "odna-statya");
    assert_eq!(second["slug"], "odna-statya-2");
}

#[tokio::test]
async fn publishing_creates_an_outbox_event_and_draft_does_not() {
    let _guard = articles_lock();
    let state = test_state().await;

    let draft = create_article(&state, "Черновик без события", "draft").await;
    let draft_id = draft["id"].as_i64().unwrap();

    let (_, outbox) = request(
        &state,
        "GET",
        "/api/admin/outbox?status=pending",
        None,
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(
        outbox["pending"], 0,
        "черновик не должен порождать события кросспостинга"
    );

    let (_, _) = request(
        &state,
        "PATCH",
        &format!("/api/admin/articles/{draft_id}"),
        Some(serde_json::json!({ "status": "published" })),
        Some(ADMIN_TOKEN),
    )
    .await;

    let (_, outbox) = request(
        &state,
        "GET",
        "/api/admin/outbox?status=pending",
        None,
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(outbox["pending"], 1);

    let item = &outbox["items"][0];
    assert_eq!(item["event_type"], "article.published");
    assert_eq!(item["payload"]["article"]["slug"], "chernovik-bez-sobytiya");
    assert_eq!(
        item["payload"]["article"]["url"],
        "https://test.inteli-dev.ru/blog/chernovik-bez-sobytiya"
    );
    // Снимок статьи: n8n получает текст, не делая второй запрос.
    assert!(item["payload"]["article"]["body_markdown"]
        .as_str()
        .unwrap_or("")
        .contains("Текст"));
}

#[tokio::test]
async fn n8n_feed_is_disabled_without_a_configured_key() {
    let _guard = articles_lock();
    let state = test_state().await;

    // Конфигурация без ключа: фид выключен целиком (404), а не «пускает всех».
    let mut config = (*state.config).clone();
    config.integrations.n8n_api_key = String::new();
    let mut bare = state.clone();
    bare.config = Arc::new(config);

    let (status, _) = request_with_headers(
        &bare,
        "GET",
        "/api/integrations/outbox",
        None,
        &[("x-api-key", N8N_KEY)],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn n8n_feed_rejects_a_wrong_key() {
    let _guard = articles_lock();
    let state = test_state().await;

    let (status, _) = request_with_headers(&state, "GET", "/api/integrations/outbox", None, &[]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = request_with_headers(
        &state,
        "GET",
        "/api/integrations/outbox",
        None,
        &[("x-api-key", "wrong-key")],
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn n8n_feed_delivers_events_and_ack_is_idempotent() {
    let _guard = articles_lock();
    let state = test_state().await;

    let article = create_article(&state, "Для кросспостинга", "published").await;
    assert_eq!(article["status"], "published");

    // 1. Фид отдаёт событие с ключом из конфига.
    let (status, feed) = request_with_headers(
        &state,
        "GET",
        "/api/integrations/outbox",
        None,
        &[("x-api-key", N8N_KEY)],
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let events = feed["events"].as_array().expect("список событий");
    assert_eq!(events.len(), 1);
    let event_id = events[0]["id"].as_i64().unwrap();
    assert_eq!(events[0]["event_type"], "article.published");
    assert_eq!(feed["base_url"], "https://test.inteli-dev.ru");

    // 2. Подтверждение доставки.
    let (status, acked) = request_with_headers(
        &state,
        "POST",
        "/api/integrations/outbox/ack",
        Some(serde_json::json!({ "ids": [event_id] })),
        &[("x-api-key", N8N_KEY)],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(acked["acked"], 1);

    // 3. Повторный ack идемпотентен: n8n может переподтвердить батч.
    let (_, again) = request_with_headers(
        &state,
        "POST",
        "/api/integrations/outbox/ack",
        Some(serde_json::json!({ "ids": [event_id] })),
        &[("x-api-key", N8N_KEY)],
    )
    .await;
    assert_eq!(again["acked"], 0);

    // 4. Очередь пуста.
    let (_, feed) = request_with_headers(
        &state,
        "GET",
        "/api/integrations/outbox",
        None,
        &[("x-api-key", N8N_KEY)],
    )
    .await;
    assert_eq!(feed["count"], 0);
}

#[tokio::test]
async fn n8n_feed_fail_counts_attempts_and_gives_up() {
    let _guard = articles_lock();
    let state = test_state().await;

    create_article(&state, "Падающая доставка", "published").await;

    let (_, feed) = request_with_headers(
        &state,
        "GET",
        "/api/integrations/outbox",
        None,
        &[("x-api-key", N8N_KEY)],
    )
    .await;
    let event_id = feed["events"][0]["id"].as_i64().unwrap();

    // Падаем больше, чем допускает лимит попыток: событие уходит в failed и
    // перестаёт выдаваться, чтобы один сбой не блокировал пайплайн навсегда.
    for _ in 0..6 {
        let (status, _) = request_with_headers(
            &state,
            "POST",
            "/api/integrations/outbox/fail",
            Some(serde_json::json!({ "id": event_id, "error": "502 от n8n" })),
            &[("x-api-key", N8N_KEY)],
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    let (_, feed) = request_with_headers(
        &state,
        "GET",
        "/api/integrations/outbox",
        None,
        &[("x-api-key", N8N_KEY)],
    )
    .await;
    assert_eq!(feed["count"], 0, "исчерпавшее попытки событие не отдаётся");

    let (_, failed) = request_with_headers(
        &state,
        "GET",
        "/api/integrations/outbox?status=failed",
        None,
        &[("x-api-key", N8N_KEY)],
    )
    .await;
    assert_eq!(failed["count"], 1);
    assert_eq!(failed["events"][0]["last_error"], "502 от n8n");
}

#[tokio::test]
async fn only_published_articles_reach_rss_and_sitemap() {
    let _guard = articles_lock();
    let state = test_state().await;

    create_article(&state, "Черновик", "draft").await;
    create_article(&state, "Публичная статья", "published").await;

    let (status, rss) = request_text(&state, "/rss.xml").await;
    assert_eq!(status, StatusCode::OK);
    assert!(rss.contains("<rss version=\"2.0\""));
    assert!(rss.contains("Публичная статья"));
    assert!(
        !rss.contains("Черновик"),
        "черновик не должен попадать в ленту: {rss}"
    );
    // Полный текст отдаётся, чтобы n8n собрал пост одним запросом.
    assert!(rss.contains("<content:encoded><![CDATA["));

    let (status, sitemap) = request_text(&state, "/sitemap.xml").await;
    assert_eq!(status, StatusCode::OK);
    assert!(sitemap.contains("<loc>https://test.inteli-dev.ru/</loc>"));
    assert!(sitemap.contains("<loc>https://test.inteli-dev.ru/blog/publichnaya-statya</loc>"));
    assert!(!sitemap.contains("chernovik"));
}

#[tokio::test]
async fn deleting_a_published_article_reports_deletion_to_n8n() {
    let _guard = articles_lock();
    let state = test_state().await;

    let article = create_article(&state, "Удаляемая", "published").await;
    let id = article["id"].as_i64().unwrap();

    let (status, _) = request(
        &state,
        "DELETE",
        &format!("/api/admin/articles/{id}"),
        None,
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Событие об удалении должно пережить саму статью: иначе n8n не узнает,
    // что материал снят с сайта.
    let (_, feed) = request_with_headers(
        &state,
        "GET",
        "/api/integrations/outbox",
        None,
        &[("x-api-key", N8N_KEY)],
    )
    .await;
    let events = feed["events"].as_array().unwrap();
    let types: Vec<&str> = events
        .iter()
        .map(|e| e["event_type"].as_str().unwrap_or(""))
        .collect();
    assert!(types.contains(&"article.deleted"), "получили {types:?}");
    assert!(events
        .iter()
        .any(|e| e["payload"]["article"]["slug"] == "udalyaemaya"));
}

#[tokio::test]
async fn markdown_preview_renders_and_neutralises_scripts() {
    let _guard = articles_lock();
    let state = test_state().await;

    let (status, body) = request(
        &state,
        "POST",
        "/api/admin/article-preview",
        Some(serde_json::json!({
            "body_markdown": "# Заголовок\n\n**жирный** <script>alert(1)</script>"
        })),
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let html = body["html"].as_str().unwrap_or("");
    assert!(html.contains("<h1>Заголовок</h1>"));
    assert!(html.contains("<strong>жирный</strong>"));
    assert!(!html.contains("<script>"), "скрипт просочился в предпросмотр: {html}");
    assert_eq!(body["reading_time_minutes"], 1);

    // Предпросмотр закрыт админским токеном, как и остальные админ-ручки.
    let (status, _) = request(
        &state,
        "POST",
        "/api/admin/article-preview",
        Some(serde_json::json!({ "body_markdown": "текст" })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unpublishing_removes_the_article_from_the_feed() {
    let _guard = articles_lock();
    let state = test_state().await;

    let article = create_article(&state, "Снимем с публикации", "published").await;
    let id = article["id"].as_i64().unwrap();

    let (_, rss) = request_text(&state, "/rss.xml").await;
    assert!(rss.contains("Снимем с публикации"));

    let (status, _) = request(
        &state,
        "PATCH",
        &format!("/api/admin/articles/{id}"),
        Some(serde_json::json!({ "status": "archived" })),
        Some(ADMIN_TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, rss) = request_text(&state, "/rss.xml").await;
    assert!(
        !rss.contains("Снимем с публикации"),
        "снятая с публикации статья осталась в ленте: {rss}"
    );

    // И n8n получает событие о снятии.
    let (_, feed) = request_with_headers(
        &state,
        "GET",
        "/api/integrations/outbox",
        None,
        &[("x-api-key", N8N_KEY)],
    )
    .await;
    let types: Vec<&str> = feed["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["event_type"].as_str().unwrap_or(""))
        .collect();
    assert!(types.contains(&"article.unpublished"), "получили {types:?}");
}

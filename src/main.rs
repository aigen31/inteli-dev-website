//! Точка входа: загрузка конфигурации → инициализация слоёв → запуск сервера.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use leptos::prelude::*;
use leptos_axum::{generate_route_list, LeptosRoutes};

use inteli_dev::api;
use inteli_dev::cache::{InMemoryCache, RateLimiter};
use inteli_dev::config::AppConfig;
use inteli_dev::error::AppResult;
use inteli_dev::llm::deepseek::DeepSeekProvider;
use inteli_dev::llm::ChatProvider;
use inteli_dev::memory::content::SiteContent;
use inteli_dev::memory::OpenVikingClient;
use inteli_dev::notification::NotificationService;
use inteli_dev::services::{ChatService, LeadService};
use inteli_dev::state::AppState;
use inteli_dev::storage;
use inteli_dev::ui::App;

#[tokio::main]
async fn main() {
    let config = match AppConfig::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[fatal] {e}");
            std::process::exit(1);
        }
    };

    init_tracing(&config.logging.level);

    if let Err(e) = run(config).await {
        tracing::error!("fatal: {e}");
        std::process::exit(1);
    }
}

fn init_tracing(level: &str) {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    // Игнорируем ошибку повторной инициализации (в тестах).
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

/// Основной жизненный цикл приложения.
async fn run(config: AppConfig) -> AppResult<()> {
    let config = Arc::new(config);
    let addr = config.socket_addr();

    // 1. Storage — SQLite (WAL).
    let db = storage::init_pool(&config.database.path).await?;

    // 2. Memory — OpenViking (с fallback на встроенный контент).
    let viking = OpenVikingClient::new(
        &config.openviking.base_url,
        Duration::from_secs(config.openviking.timeout_seconds),
    );
    let content = SiteContent::load(&viking).await;
    SiteContent::set_global(content);

    // 3. LLM — DeepSeek V4 (OpenAI-compatible).
    let llm: Arc<dyn ChatProvider> = Arc::new(DeepSeekProvider::new(
        config.llm.api_key.clone(),
        config.llm.base_url.clone(),
        config.llm.model.clone(),
    ));

    // 4. Cache + rate limiting.
    let cache = Arc::new(InMemoryCache::new(Duration::from_secs(
        config.cache.ttl_seconds,
    )));
    let rate_limiter = Arc::new(RateLimiter::new(
        config.ratelimit.max_requests_per_minute,
        Duration::from_secs(config.ratelimit.window_seconds),
    ));

    // 5. Уведомления и сервисы.
    let notifications = Arc::new(NotificationService::new(
        config.telegram.admin_chat_id,
        config.telegram.bot_token.clone(),
        config.vk.oauth_token.clone(),
        config.vk.admin_user_id,
    ));
    let chat = Arc::new(ChatService::new(
        llm,
        Arc::new(viking),
        cache.clone(),
        db.clone(),
    ));
    let leads = Arc::new(LeadService::new(db.clone(), notifications.clone()));

    // 6. Leptos-настройки (для SSR-роутера).
    let leptos_options = build_leptos_options(addr);

    let state = AppState {
        config,
        db,
        chat,
        leads,
        notifications,
        cache,
        rate_limiter,
        leptos_options: leptos_options.clone(),
        started_at: Instant::now(),
    };

    // 7. Собираем router: API + Leptos SSR + fallback.
    let routes = generate_route_list(App);
    let app: Router = api::routes()
        .leptos_routes(&state, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
        .with_state(state);

    tracing::info!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn build_leptos_options(addr: std::net::SocketAddr) -> LeptosOptions {
    match get_configuration(None) {
        Ok(conf) => {
            let mut options = conf.leptos_options;
            options.site_addr = addr;
            options
        }
        Err(e) => {
            tracing::error!("Leptos configuration failed: {e}");
            std::process::exit(1);
        }
    }
}

/// HTML-обёртка всех страниц (SSR).
fn shell(_options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="ru">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="description" content="Fullstack-разработчик и архитектор приватных AI-систем: локальный инференс, MCP-серверы, голосовые ассистенты, PHP + JS."/>
                <title>{"inteli.dev — Fullstack & приватные AI-системы"}</title>
                <link rel="stylesheet" href="/assets/style.css"/>
                <link rel="manifest" href="/manifest.json"/>
                <link rel="icon" href="/favicon.svg" type="image/svg+xml"/>
                <script src="/assets/main.js" defer></script>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

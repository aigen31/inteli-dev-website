//! Общее состояние приложения, разделяемое между axum-хендлерами и Leptos-SSR.
//!
//! Хранится в `Router` как `axum::extract::State`. Реализует `FromRef<AppState>`
//! для `LeptosOptions`, чтобы `leptos_axum` мог извлекать свои настройки из
//! нашего состояния (стандартный паттерн интеграции Leptos + axum).

use std::sync::Arc;
use std::time::Instant;

use axum::extract::FromRef;
use leptos::config::LeptosOptions;
use sqlx::sqlite::SqlitePool;

use crate::cache::{InMemoryCache, RateLimiter};
use crate::config::AppConfig;
use crate::notification::NotificationService;
use crate::services::{ChatService, LeadService};

/// Состояние приложения (клонируется дешёво — все поля `Arc`/`Clone`).
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: SqlitePool,
    pub chat: Arc<ChatService>,
    pub leads: Arc<LeadService>,
    pub notifications: Arc<NotificationService>,
    pub cache: Arc<InMemoryCache>,
    pub rate_limiter: Arc<RateLimiter>,
    pub leptos_options: LeptosOptions,
    pub started_at: Instant,
}

impl FromRef<AppState> for LeptosOptions {
    fn from_ref(state: &AppState) -> Self {
        state.leptos_options.clone()
    }
}

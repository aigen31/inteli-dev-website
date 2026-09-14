//! Загрузка и валидация конфигурации приложения.
//!
//! Источник конфигурации — `config.toml` (структура секций), дополняемый
//! переменными окружения для секретов (`LLM_API_KEY`, `TELEGRAM_BOT_TOKEN`,
//! `ADMIN_TOKEN` и т.д.). Секреты никогда не хранятся в коде и не коммитятся.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use serde::Deserialize;

use crate::error::{AppError, AppResult};

/// Полная конфигурация приложения.
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub cache: CacheConfig,
    pub openviking: OpenVikingConfig,
    pub llm: LlmConfig,
    pub telegram: TelegramConfig,
    pub vk: VkConfig,
    pub admin: AdminConfig,
    pub ratelimit: RateLimitConfig,
    pub logging: LoggingConfig,
    pub security: SecurityConfig,
    /// Пользовательские лимиты. Секция опциональна: без неё берутся значения
    /// по умолчанию, поэтому старый config.toml не ломает запуск.
    #[serde(default)]
    pub limits: crate::limits::Limits,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_read_timeout_ms")]
    pub read_timeout_ms: u64,
    #[serde(default = "default_write_timeout_ms")]
    pub write_timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CacheConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_ttl")]
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenVikingConfig {
    pub base_url: String,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_read_timeout")]
    pub read_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    /// API key — заполняется из env `LLM_API_KEY`, не из config.toml.
    #[serde(default)]
    pub api_key: String,
    /// Потолок выходных токенов. Системный промпт требует «1-2 предложения +
    /// максимум 3 пункта», поэтому 600 — с большим запасом. Нужен только чтобы
    /// обуздать «разболтавшуюся» модель и не платить за простыню.
    #[serde(default = "default_llm_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_llm_temperature")]
    pub temperature: f32,
    /// Сколько результатов семантического поиска подмешивать в промпт.
    #[serde(default = "default_context_top_k")]
    pub context_top_k: usize,
    /// Сколько символов брать из каждого результата.
    #[serde(default = "default_context_snippet_chars")]
    pub context_snippet_chars: usize,
    /// Общий бюджет RAG-контекста в символах — жёсткий потолок входа.
    #[serde(default = "default_context_max_chars")]
    pub context_max_chars: usize,
    /// Порог релевантности: результаты с меньшим score отбрасываются.
    #[serde(default = "default_context_min_score")]
    pub context_min_score: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramConfig {
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub admin_chat_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VkConfig {
    #[serde(default)]
    pub oauth_token: String,
    #[serde(default)]
    pub admin_user_id: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminConfig {
    #[serde(default)]
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_max_requests")]
    pub max_requests_per_minute: u32,
    #[serde(default = "default_window")]
    pub window_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    #[serde(default = "default_hash_algo")]
    pub ip_hash_algorithm: String,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    8080
}
fn default_read_timeout_ms() -> u64 {
    5000
}
fn default_write_timeout_ms() -> u64 {
    10000
}
fn default_true() -> bool {
    true
}
fn default_ttl() -> u64 {
    3600
}
fn default_timeout() -> u64 {
    10
}
fn default_read_timeout() -> u64 {
    5
}
fn default_max_requests() -> u32 {
    10
}
fn default_window() -> u64 {
    60
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_hash_algo() -> String {
    "sha256".to_string()
}
fn default_llm_max_tokens() -> u32 {
    600
}
fn default_llm_temperature() -> f32 {
    0.7
}
fn default_context_top_k() -> usize {
    4
}
fn default_context_snippet_chars() -> usize {
    400
}
fn default_context_max_chars() -> usize {
    1600
}
fn default_context_min_score() -> f64 {
    0.35
}

impl AppConfig {
    /// Загружает конфигурацию из файла и применяет env-переопределения для секретов.
    ///
    /// Путь к файлу берётся из `CONFIG_PATH` (по умолчанию `config.toml`).
    pub fn load() -> AppResult<Self> {
        let path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());

        let raw = std::fs::read_to_string(&path).map_err(|e| {
            AppError::Config(format!(
                "cannot read config file `{path}` (set CONFIG_PATH or create config.toml): {e}"
            ))
        })?;

        let mut config: AppConfig = toml::from_str(&raw)
            .map_err(|e| AppError::Config(format!("invalid config file `{path}`: {e}")))?;

        config.apply_env_overrides();
        Ok(config)
    }

    /// Переопределяет значения из переменных окружения (приоритет над config.toml).
    ///
    /// Секреты (LLM API key, токены ботов, admin token) читаются ТОЛЬКО из env.
    fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("DB_PATH") {
            self.database.path = v;
        }
        if let Ok(v) = std::env::var("LLM_API_KEY") {
            self.llm.api_key = v;
        }
        if let Ok(v) = std::env::var("LLM_BASE_URL") {
            self.llm.base_url = v;
        }
        if let Ok(v) = std::env::var("LLM_MODEL") {
            self.llm.model = v;
        }
        if let Ok(v) = std::env::var("OPENVIKING_BASE_URL") {
            self.openviking.base_url = v;
        }
        if let Ok(v) = std::env::var("TELEGRAM_BOT_TOKEN") {
            self.telegram.bot_token = v;
        }
        if let Ok(v) = std::env::var("TELEGRAM_ADMIN_CHAT_ID") {
            if let Ok(parsed) = v.parse() {
                self.telegram.admin_chat_id = parsed;
            }
        }
        if let Ok(v) = std::env::var("VK_OAUTH_TOKEN") {
            self.vk.oauth_token = v;
        }
        if let Ok(v) = std::env::var("VK_ADMIN_USER_ID") {
            if let Ok(parsed) = v.parse() {
                self.vk.admin_user_id = parsed;
            }
        }
        if let Ok(v) = std::env::var("ADMIN_TOKEN") {
            self.admin.token = v;
        }
        if let Ok(v) = std::env::var("RUST_LOG") {
            self.logging.level = v;
        }
        if let Ok(v) = std::env::var("PORT") {
            if let Ok(parsed) = v.parse() {
                self.server.port = parsed;
            }
        }
    }

    /// SocketAddr для HTTP-сервера.
    pub fn socket_addr(&self) -> SocketAddr {
        let ip: IpAddr = self
            .server
            .host
            .parse()
            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        SocketAddr::new(ip, self.server.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_toml() -> &'static str {
        r#"
[server]
host = "127.0.0.1"
port = 9000

[database]
path = "/tmp/test.db"

[cache]
enabled = true
ttl_seconds = 60

[openviking]
base_url = "http://localhost:1933/mcp"

[llm]
provider = "deepseek"
base_url = "https://api.deepseek.com/v1"
model = "deepseek-chat"

[telegram]
bot_token = ""
admin_chat_id = 0

[vk]
oauth_token = ""
admin_user_id = 0

[admin]
token = ""

[ratelimit]
max_requests_per_minute = 5
window_seconds = 30

[logging]
level = "debug"

[security]
ip_hash_algorithm = "sha256"
"#
    }

    #[test]
    fn parses_full_config() {
        let cfg: AppConfig = toml::from_str(sample_toml()).expect("valid toml");
        assert_eq!(cfg.server.port, 9000);
        assert_eq!(cfg.database.path, "/tmp/test.db");
        assert_eq!(cfg.ratelimit.max_requests_per_minute, 5);
        assert_eq!(cfg.logging.level, "debug");
        assert_eq!(cfg.cache.ttl_seconds, 60);
    }

    #[test]
    fn defaults_are_applied() {
        let cfg: AppConfig = toml::from_str(
            r#"
[server]
[database]
path = "x.db"
[cache]
[openviking]
base_url = "http://x"
[llm]
provider = "deepseek"
base_url = "http://x"
model = "m"
[telegram]
[vk]
[admin]
[ratelimit]
[logging]
[security]
"#,
        )
        .expect("minimal toml");

        assert_eq!(cfg.server.host, "0.0.0.0");
        assert_eq!(cfg.server.port, 8080);
        assert!(cfg.cache.enabled);
        assert_eq!(cfg.cache.ttl_seconds, 3600);
        assert_eq!(cfg.ratelimit.max_requests_per_minute, 10);
        assert_eq!(cfg.security.ip_hash_algorithm, "sha256");
    }
}

//! Загрузка и валидация конфигурации приложения.
//!
//! Источник конфигурации — `config.toml` (структура секций), дополняемый
//! переменными окружения для секретов (`LLM_API_KEY`, `TELEGRAM_BOT_TOKEN`,
//! `ADMIN_TOKEN` и т.д.). Секреты никогда не хранятся в коде и не коммитятся.

use std::collections::BTreeMap;
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
    /// Бот настроек сайта. Секция опциональна: без неё эндпоинт выключен.
    #[serde(default)]
    pub settings_bot: SettingsBotConfig,
    pub vk: VkConfig,
    pub admin: AdminConfig,
    /// Статистика GitHub для блока «Открытый код». Секция опциональна.
    #[serde(default)]
    pub github: GitHubConfig,
    pub ratelimit: RateLimitConfig,
    pub logging: LoggingConfig,
    pub security: SecurityConfig,
    /// Внешние интеграции (фид событий для n8n). Секция опциональна.
    #[serde(default)]
    pub integrations: IntegrationsConfig,
    /// SEO-поверхность: файлы в корне и IndexNow. Секция опциональна.
    #[serde(default)]
    pub seo: SeoConfig,
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
    /// Публичный адрес сайта: из него собираются абсолютные ссылки в RSS,
    /// sitemap и событиях кросспостинга. Отдельно от `host`/`port`, потому что
    /// приложение слушает `0.0.0.0:8080` за обратным прокси.
    #[serde(default = "default_public_url")]
    pub public_url: String,
    #[serde(default = "default_read_timeout_ms")]
    pub read_timeout_ms: u64,
    #[serde(default = "default_write_timeout_ms")]
    pub write_timeout_ms: u64,
}

/// Внешние интеграции. Секция опциональна: без неё кросспостинг выключен.
///
/// Сама доставка в n8n здесь не реализована — сайт только выставляет наружу
/// ленту событий (transactional outbox). Читатель забирает её сам, поэтому
/// адрес n8n сайту знать не нужно. См. `docs/articles.md`.
#[derive(Debug, Clone, Deserialize)]
pub struct IntegrationsConfig {
    /// Ключ доступа к фиду событий. Секрет: только из env `N8N_API_KEY`.
    ///
    /// Пустая строка выключает фид целиком (fail closed): эндпоинт отвечает
    /// 404, а не «пускает всех без пароля».
    #[serde(default)]
    pub n8n_api_key: String,
    /// Сколько событий отдавать за один запрос.
    #[serde(default = "default_outbox_batch_limit")]
    pub outbox_batch_limit: i64,
}

impl Default for IntegrationsConfig {
    fn default() -> Self {
        Self {
            n8n_api_key: String::new(),
            outbox_batch_limit: default_outbox_batch_limit(),
        }
    }
}

impl IntegrationsConfig {
    /// Фид событий доступен: ключ задан и непустой.
    pub fn is_active(&self) -> bool {
        !self.n8n_api_key.trim().is_empty()
    }
}

/// SEO-поверхность сайта: файлы в корне и IndexNow.
///
/// robots.txt здесь нет намеренно: он собирается на лету из `public_url`
/// (см. `services::seo`), чтобы адрес карты сайта не мог разойтись с адресом,
/// который отдаёт `/sitemap.xml`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SeoConfig {
    /// Ключ IndexNow. Не секрет — он же в URL запроса и в имени файла, — но
    /// задаётся из env `INDEXNOW_KEY`, чтобы менять и отзывать без пересборки.
    /// Пустое значение выключает протокол целиком.
    #[serde(default)]
    pub indexnow_key: String,
    /// Дополнительные файлы в корне сайта: имя → содержимое.
    ///
    /// Нужны для подтверждения прав на сайт (Яндекс и Google отдают файл вида
    /// `googleXXXX.html` / `yandex_XXXX.html` с проверочной строкой внутри).
    /// Подтверждение через DNS TXT дешевле — этот вариант для случаев, когда
    /// DNS недоступен. Правка конфига применяется после рестарта контейнера.
    #[serde(default)]
    pub root_files: BTreeMap<String, String>,
}

impl SeoConfig {
    /// Файлы, которые надо отдать из корня: явные плюс файл ключа IndexNow.
    ///
    /// Если ключ задан, файл `{ключ}.txt` добавляется сам — по спецификации
    /// IndexNow поисковик проверяет ключ именно так.
    pub fn root_files(&self) -> Vec<(String, String)> {
        let mut files: Vec<(String, String)> = self
            .root_files
            .iter()
            .map(|(name, content)| (name.clone(), content.clone()))
            .collect();

        if let Some(key_file) = crate::services::seo::indexnow_key_file(&self.indexnow_key) {
            if !self.root_files.contains_key(&key_file.0) {
                files.push(key_file);
            }
        }

        files
    }

    /// IndexNow включён: ключ задан и проходит проверку спецификации.
    pub fn is_indexnow_active(&self) -> bool {
        crate::services::seo::is_valid_indexnow_key(self.indexnow_key.trim())
    }

    /// Проверяет секцию до старта сервера.
    ///
    /// Ошибка здесь лучше паники: имя файла из конфига становится маршрутом, и
    /// столкновение с существующим (например, `robots.txt`) уронило бы
    /// приложение уже на сборке роутера.
    pub fn validate(&self) -> AppResult<()> {
        let key = self.indexnow_key.trim();
        if !key.is_empty() && !crate::services::seo::is_valid_indexnow_key(key) {
            return Err(AppError::Config(format!(
                "[seo].indexnow_key: ключ должен быть длиной 8–128 символов \
                 из латиницы, цифр и дефиса (получено {} символов)",
                key.len()
            )));
        }

        for (name, content) in &self.root_files {
            if !crate::services::seo::is_valid_root_file_name(name) {
                return Err(AppError::Config(format!(
                    "[seo].root_files: недопустимое имя файла `{name}`. \
                     Нужен один сегмент пути с расширением, из латиницы, цифр, \
                     точки, дефиса и подчёркивания, не начинающийся с точки, \
                     и не занятый статикой сайта"
                )));
            }
            if content.trim().is_empty() {
                return Err(AppError::Config(format!(
                    "[seo].root_files: содержимое файла `{name}` пустое — \
                     подтверждение прав не сработает"
                )));
            }
            if content.len() > 4096 {
                return Err(AppError::Config(format!(
                    "[seo].root_files: содержимое файла `{name}` длиннее 4096 \
                     символов — это не файл подтверждения прав"
                )));
            }
        }

        // Явный файл с именем ключа IndexNow отдал бы поисковику не тот текст,
        // и проверка ключа молча провалилась бы.
        if let Some((key_file, _)) = crate::services::seo::indexnow_key_file(key) {
            if self.root_files.contains_key(&key_file) {
                return Err(AppError::Config(format!(
                    "[seo].root_files: файл `{key_file}` совпадает с файлом ключа \
                     IndexNow. Уберите его из root_files — ключ отдаётся \
                     автоматически"
                )));
            }
        }

        Ok(())
    }
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

/// Отдельный Telegram-бот для управления настройками сайта.
///
/// Это **не** тот бот, что присылает заявки: разные токены, разные права.
/// Бот настроек умеет менять состояние сайта (статус занятости), поэтому
/// доступ к нему ограничен одним Telegram-ID и секретом вебхука.
#[derive(Debug, Clone, Deserialize)]
pub struct SettingsBotConfig {
    /// Токен бота. Секрет: только из env `SETTINGS_BOT_TOKEN`.
    #[serde(default)]
    pub token: String,
    /// Telegram user id владельца. Все остальные получают отказ.
    /// Секрет: только из env `SETTINGS_BOT_ADMIN_ID`.
    #[serde(default)]
    pub admin_id: i64,
    /// Значение, которое Telegram присылает в заголовке
    /// `X-Telegram-Bot-Api-Secret-Token`. Пустая строка выключает вебхук
    /// целиком (fail closed): без секрета эндпоинт смог бы дёргать кто угодно.
    #[serde(default)]
    pub webhook_secret: String,
    /// Базовый адрес Bot API. Меняется только в тестах (чтобы не ходить в
    /// сеть) и при работе через прокси: api.telegram.org в РФ бывает недоступен.
    #[serde(default = "default_telegram_api_base")]
    pub api_base: String,
}

impl SettingsBotConfig {
    /// Бот готов принимать команды: есть токен, владелец и секрет вебхука.
    pub fn is_active(&self) -> bool {
        !self.token.is_empty() && self.admin_id != 0 && !self.webhook_secret.is_empty()
    }
}

impl Default for SettingsBotConfig {
    fn default() -> Self {
        Self {
            token: String::new(),
            admin_id: 0,
            webhook_secret: String::new(),
            api_base: default_telegram_api_base(),
        }
    }
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

/// Статистика GitHub для блока «Открытый код» на главной.
///
/// Публичные данные (репозитории, языки) доступны анонимно, поэтому блок
/// работает без токена. Токен нужен только для двух вещей: поднять лимит
/// GitHub API с 60 до 5000 запросов в час на IP и получать точный счётчик
/// контрибуций через GraphQL (включая приватные, если они разрешены в
/// настройках профиля). См. `docs/github-stats.md`.
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubConfig {
    /// Логин на GitHub. Пустая строка полностью выключает блок.
    #[serde(default = "default_github_username")]
    pub username: String,
    /// Personal Access Token. Секрет: приходит из env `GITHUB_TOKEN`.
    #[serde(default)]
    pub token: String,
    /// Показывать ли блок. Позволяет выключить без удаления логина.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Как часто обновлять статистику. GitHub-статистика меняется медленно,
    /// а лимит анонимных запросов общий на IP сервера — 6 часов безопасно.
    #[serde(default = "default_github_cache_ttl")]
    pub cache_ttl_seconds: u64,
    /// Показывать ли звёзды и подписчиков. По умолчанию выключено: маленькие
    /// числа («2 звезды», «1 подписчик») на странице продаж вредят больше,
    /// чем помогают. Включайте, когда числа станут приличными.
    #[serde(default)]
    pub show_stars: bool,
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
fn default_telegram_api_base() -> String {
    "https://api.telegram.org".to_string()
}
fn default_github_username() -> String {
    "aigen31".to_string()
}
fn default_github_cache_ttl() -> u64 {
    21600
}
fn default_public_url() -> String {
    "https://inteli-dev.ru".to_string()
}
fn default_outbox_batch_limit() -> i64 {
    50
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            username: default_github_username(),
            token: String::new(),
            enabled: default_true(),
            cache_ttl_seconds: default_github_cache_ttl(),
            show_stars: false,
        }
    }
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
        config.seo.validate()?;
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
        // Бот настроек сайта: и токен, и id владельца — секреты, только из env.
        if let Ok(v) = std::env::var("SETTINGS_BOT_TOKEN") {
            if !v.trim().is_empty() {
                self.settings_bot.token = v;
            }
        }
        if let Ok(v) = std::env::var("SETTINGS_BOT_ADMIN_ID") {
            if let Ok(parsed) = v.trim().parse() {
                self.settings_bot.admin_id = parsed;
            }
        }
        if let Ok(v) = std::env::var("SETTINGS_BOT_WEBHOOK_SECRET") {
            if !v.trim().is_empty() {
                self.settings_bot.webhook_secret = v;
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
        if let Ok(v) = std::env::var("GITHUB_TOKEN") {
            self.github.token = v;
        }
        // Публичный адрес сайта: в проде — https://inteli-dev.ru, в локальной
        // проверке — http://127.0.0.1:8282, чтобы ссылки в RSS и событиях вели
        // туда, где сайт реально открывается.
        if let Ok(v) = std::env::var("PUBLIC_URL") {
            if !v.trim().is_empty() {
                self.server.public_url = v.trim().trim_end_matches('/').to_string();
            }
        }
        // Ключ фида событий для n8n: секрет, только из env.
        if let Ok(v) = std::env::var("N8N_API_KEY") {
            if !v.trim().is_empty() {
                self.integrations.n8n_api_key = v;
            }
        }
        // Ключ IndexNow. Не секрет, но из env — чтобы включить протокол на
        // сервере без пересборки образа. Пустое значение выключает его.
        if let Ok(v) = std::env::var("INDEXNOW_KEY") {
            if !v.trim().is_empty() {
                self.seo.indexnow_key = v.trim().to_string();
            }
        }
        if let Ok(v) = std::env::var("GITHUB_USERNAME") {
            // Пустое значение игнорируем: docker-compose передаёт переменную
            // всегда (${GITHUB_USERNAME:-}), и пустая строка иначе выключила бы
            // блок «Открытый код» вопреки config.toml.
            if !v.trim().is_empty() {
                self.github.username = v;
            }
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
        // Секция [github] опциональна: без неё блок «Открытый код» включён
        // для дефолтного логина, но звёзды/подписчики не показываются.
        assert!(cfg.github.enabled);
        assert_eq!(cfg.github.username, "aigen31");
        assert!(cfg.github.token.is_empty());
        assert!(!cfg.github.show_stars);
        // Адрес сайта и интеграции тоже опциональны: без секций работают
        // значения по умолчанию, а фид событий остаётся выключенным.
        assert_eq!(cfg.server.public_url, "https://inteli-dev.ru");
        assert_eq!(cfg.integrations.outbox_batch_limit, 50);
        assert!(!cfg.integrations.is_active());
        // SEO-секция тоже опциональна: без неё IndexNow выключен, а файлов в
        // корне нет — сайт отдаёт только свою статику.
        assert!(!cfg.seo.is_indexnow_active());
        assert!(cfg.seo.root_files().is_empty());
        cfg.seo.validate().expect("пустая секция валидна");
    }

    #[test]
    fn integrations_section_is_parsed() {
        let cfg: AppConfig = toml::from_str(
            r#"
[server]
public_url = "http://127.0.0.1:8282/"
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

[integrations]
n8n_api_key = "secret-key"
outbox_batch_limit = 10
"#,
        )
        .expect("toml with integrations");

        assert_eq!(cfg.server.public_url, "http://127.0.0.1:8282/");
        assert_eq!(cfg.integrations.n8n_api_key, "secret-key");
        assert_eq!(cfg.integrations.outbox_batch_limit, 10);
        assert!(cfg.integrations.is_active());
    }

    /// Конфигурация с секцией `[seo]` (минимальный остальной набор).
    fn config_with_seo(seo: &str) -> AppConfig {
        toml::from_str(&format!(
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

{seo}
"#
        ))
        .expect("toml with seo")
    }

    #[test]
    fn seo_section_serves_the_key_file_and_verification_files() {
        let cfg = config_with_seo(
            r#"
[seo]
indexnow_key = "0123456789abcdef"

[seo.root_files]
"google1234abcd.html" = "google-site-verification: google1234abcd.html"
"#,
        );

        assert!(cfg.seo.is_indexnow_active());
        cfg.seo.validate().expect("валидная секция");

        let files = cfg.seo.root_files();
        // Файл ключа добавляется сам, подтверждение прав — из конфига.
        assert!(files
            .iter()
            .any(|(name, body)| name == "0123456789abcdef.txt" && body == "0123456789abcdef"));
        assert!(files.iter().any(|(name, _)| name == "google1234abcd.html"));
    }

    #[test]
    fn seo_validation_rejects_configs_that_would_break_the_router() {
        // Имя, занятое статикой: маршрут столкнулся бы с `/robots.txt`, и
        // приложение упало бы на сборке роутера.
        let reserved = config_with_seo(
            r#"
[seo.root_files]
"robots.txt" = "User-agent: *"
"#,
        );
        assert!(reserved.seo.validate().is_err());

        // Путь наружу корня.
        let traversal = config_with_seo(
            r#"
[seo.root_files]
"../secret.html" = "x"
"#,
        );
        assert!(traversal.seo.validate().is_err());

        // Слишком короткий ключ подбирается, а значит уведомления мог бы слать кто угодно.
        let short_key = config_with_seo(
            r#"
[seo]
indexnow_key = "abc"
"#,
        );
        assert!(short_key.seo.validate().is_err());

        // Пустое содержимое файла подтверждения.
        let empty = config_with_seo(
            r#"
[seo.root_files]
"google1234abcd.html" = "   "
"#,
        );
        assert!(empty.seo.validate().is_err());
    }

    #[test]
    fn seo_validation_rejects_a_hand_written_key_file() {
        // Файл с именем ключа, заданный руками, отдал бы поисковику чужой текст.
        let cfg = config_with_seo(
            r#"
[seo]
indexnow_key = "0123456789abcdef"

[seo.root_files]
"0123456789abcdef.txt" = "не тот текст"
"#,
        );

        let err = cfg.seo.validate().expect_err("должна быть ошибка");
        assert!(
            err.to_string().contains("0123456789abcdef.txt"),
            "ошибка должна называть файл: {err}"
        );
    }
}

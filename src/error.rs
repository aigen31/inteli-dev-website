//! Единая система ошибок приложения.
//!
//! Все слои (HTTP → Services → Memory → Storage → LLM) возвращают [`AppError`],
//! что позволяет конвертировать их в HTTP-ответы в одном месте (`api::error`).

use thiserror::Error;

/// Универсальный тип ошибки приложения.
///
/// Использует `thiserror` для автоматической реализации `Display` и `From`.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("LLM API error ({status}): {body}")]
    LlmApiError { status: u16, body: String },

    #[error("empty LLM response")]
    EmptyLlmResponse,

    #[error("OpenViking error: {0}")]
    OpenViking(String),

    #[error("Telegram API error ({status}): {body}")]
    TelegramError { status: u16, body: String },

    #[error("VK API error ({code}): {message}")]
    VkError { code: i64, message: String },

    #[error("rate limit exceeded, retry after {retry_after}s")]
    RateLimitExceeded { retry_after: u64 },

    #[error("not found: {0}")]
    NotFound(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("internal error: {0}")]
    Internal(String),
}

/// Сокращённый псевдоним для результата приложения.
pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    /// HTTP-статус, соответствующий ошибке.
    ///
    /// Используется для конвертации `AppError` в HTTP-ответ.
    pub fn http_status(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::RateLimitExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,
            // Ошибка апстрима (LLM/Telegram/VK) для клиента = 502, не его вина.
            AppError::LlmApiError { .. }
            | AppError::TelegramError { .. }
            | AppError::VkError { .. }
            | AppError::OpenViking(_) => StatusCode::BAD_GATEWAY,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Ошибка транспорта при обращении к Telegram.
    ///
    /// `reqwest::Error` в `Display` печатает полный URL запроса, а в URL Bot API
    /// лежит токен бота (`/bot<token>/sendMessage`). Логи читают через
    /// `docker logs`, поэтому токен туда попадать не должен — оставляем только
    /// класс ошибки.
    pub fn telegram_transport(e: &reqwest::Error) -> Self {
        let kind = if e.is_timeout() {
            "таймаут запроса"
        } else if e.is_connect() {
            "не удалось подключиться (сеть или прокси)"
        } else if e.is_decode() {
            "не удалось разобрать ответ"
        } else if e.is_body() {
            "обрыв при передаче тела"
        } else {
            "ошибка транспорта"
        };
        AppError::TelegramError {
            status: 0,
            body: kind.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Любая ошибка транспорта не должна раскрывать токен.
    fn assert_no_token(error: &AppError) {
        let text = error.to_string();
        assert!(
            !text.contains("bot") || !text.contains("token"),
            "в ошибке не должно быть URL с токеном: {text}"
        );
    }

    #[test]
    fn telegram_transport_classifies_without_leaking_url() {
        // Собрать reqwest::Error без сети нельзя, поэтому проверяем через
        // реальный недостижимый адрес — соединение отбивается мгновенно.
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(300))
            .build()
            .unwrap();

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let err = runtime
            .block_on(async {
                client
                    .post("https://api.telegram.org/bot123456:SECRET/sendMessage")
                    .send()
                    .await
            })
            .expect_err("запрос не должен пройти");

        let mapped = AppError::telegram_transport(&err);
        let text = mapped.to_string();
        assert!(!text.contains("SECRET"), "токен утёк в текст ошибки: {text}");
        assert!(!text.contains("api.telegram.org"), "URL утёк: {text}");
        assert_no_token(&mapped);
    }
}

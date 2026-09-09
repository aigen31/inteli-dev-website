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
}

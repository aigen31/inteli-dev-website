//! LLM Layer — абстракция провайдеров и системные промпты.
//!
//! См. `ai-fabric.md`. Основной провайдер — DeepSeek V4 (OpenAI-compatible).

pub mod deepseek;
pub mod prompt;

use async_trait::async_trait;

use crate::error::AppResult;

/// Сообщение для LLM API.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: String, // "system" | "user" | "assistant"
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}

/// Общий интерфейс для всех LLM-провайдеров.
///
/// `#[async_trait]` делает трейт объектно-безопасным, чтобы использовать
/// `Arc<dyn ChatProvider>` (иначе async-методы дают неименованный future-тип).
#[async_trait]
pub trait ChatProvider: Send + Sync {
    /// Чат с историей сообщений. Возвращает текст ответа ассистента.
    async fn chat_with_history(&self, messages: Vec<Message>) -> AppResult<String>;

    /// Проверка доступности провайдера (по умолчанию считается доступным).
    async fn health_check(&self) -> AppResult<()> {
        Ok(())
    }

    /// Удобная обёртка: системный промпт + сообщение пользователя.
    async fn chat_with_system(&self, system: String, user: String) -> AppResult<String> {
        self.chat_with_history(vec![Message::system(system), Message::user(user)])
            .await
    }
}

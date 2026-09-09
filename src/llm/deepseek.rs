//! Провайдер DeepSeek V4 через OpenAI-compatible API.

use std::time::Duration;

use async_trait::async_trait;

use crate::error::{AppError, AppResult};
use crate::llm::{ChatProvider, Message};

/// DeepSeek (или любой OpenAI-compatible) провайдер.
pub struct DeepSeekProvider {
    api_key: String,
    base_url: String,
    model: String,
    temperature: f32,
    client: reqwest::Client,
}

impl DeepSeekProvider {
    /// Создаёт провайдер. `api_key` пустой => провайдер не сможет делать запросы.
    pub fn new(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client build cannot fail with valid config");

        Self {
            api_key: api_key.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            model: model.into(),
            temperature: 0.7,
            client,
        }
    }

    /// Есть ли ключ для реальных запросов.
    pub fn has_credentials(&self) -> bool {
        !self.api_key.is_empty()
    }
}

#[async_trait]
impl ChatProvider for DeepSeekProvider {
    async fn chat_with_history(&self, messages: Vec<Message>) -> AppResult<String> {
        if !self.has_credentials() {
            return Err(AppError::LlmApiError {
                status: 401,
                body: "LLM API key is not configured".to_string(),
            });
        }

        let payload = serde_json::json!({
            "model": self.model,
            "messages": messages
                .iter()
                .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
                .collect::<Vec<_>>(),
            "temperature": self.temperature,
            "max_tokens": 1500,
        });

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(AppError::LlmApiError {
                status: status.as_u16(),
                body: error_body,
            });
        }

        let result: serde_json::Value = response.json().await?;

        result["choices"][0]["message"]["content"]
            .as_str()
            .map(str::to_string)
            .ok_or(AppError::EmptyLlmResponse)
    }
}

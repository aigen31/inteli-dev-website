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
    max_tokens: u32,
    client: reqwest::Client,
}

impl DeepSeekProvider {
    /// Создаёт провайдер. `api_key` пустой => провайдер не сможет делать запросы.
    ///
    /// Параметры генерации берутся по умолчанию — их можно переопределить
    /// через [`with_generation`](Self::with_generation) из конфига.
    pub fn new(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        let client = reqwest::Client::builder()
            // Локальные GGUF-модели (llama.cpp) могут генерировать долго —
            // даём запас (120s), чтобы запрос не падал в fallback по таймауту.
            .timeout(Duration::from_secs(120))
            .build()
            .expect("reqwest client build cannot fail with valid config");

        Self {
            api_key: api_key.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            model: model.into(),
            temperature: 0.7,
            max_tokens: 600,
            client,
        }
    }

    /// Переопределяет температуру и потолок выходных токенов.
    pub fn with_generation(mut self, max_tokens: u32, temperature: f32) -> Self {
        self.max_tokens = max_tokens;
        self.temperature = temperature;
        self
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
            "max_tokens": self.max_tokens,
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

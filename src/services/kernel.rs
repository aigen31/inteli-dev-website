//! Client for the central Kiki assistant kernel.
//!
//! The site remains the owner of prices, content, status and leads. It calls
//! the kernel for the model, memory and tool loop, and exposes its own typed
//! tools to the kernel through `/internal/assistant`.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::KernelConfig;
use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct KernelClient {
    base_url: String,
    token: String,
    channel: String,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct KernelChatRequest {
    channel: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    conversation_id: Option<String>,
    actor: KernelActor,
    data_class: String,
    #[serde(skip_serializing_if = "Value::is_null")]
    context: Value,
}

#[derive(Debug, Serialize)]
struct KernelActor {
    id: String,
    role: String,
}

#[derive(Debug, Deserialize)]
pub struct KernelChatResponse {
    pub answer: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub suggested_next: Vec<KernelSuggestion>,
    #[serde(default)]
    pub conversation_id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct KernelSuggestion {
    pub text: String,
    #[serde(rename = "type", default)]
    pub kind: String,
}

impl KernelClient {
    pub fn new(config: &KernelConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds.max(5)))
            .build()
            .expect("kernel HTTP client must build");
        Self {
            base_url: config.url.trim().trim_end_matches('/').to_string(),
            token: config.token.clone(),
            channel: config.channel.clone(),
            client,
        }
    }

    pub async fn chat(
        &self,
        message: &str,
        actor_id: &str,
        conversation_id: Option<&str>,
        context: Value,
    ) -> AppResult<KernelChatResponse> {
        let body = KernelChatRequest {
            channel: self.channel.clone(),
            message: message.to_string(),
            conversation_id: conversation_id.map(str::to_string),
            actor: KernelActor {
                id: actor_id.to_string(),
                role: if self.channel == "site" { "visitor" } else { "user" }.to_string(),
            },
            data_class: if self.channel == "site" { "public" } else { "internal" }.to_string(),
            context,
        };

        let response = self
            .client
            .post(format!("{}/v1/chat", self.base_url))
            .bearer_auth(&self.token)
            .header("x-kernel-channel", &self.channel)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Kernel(format!(
                "kernel returned {status}: {}",
                body.chars().take(300).collect::<String>()
            )));
        }

        response
            .json::<KernelChatResponse>()
            .await
            .map_err(AppError::from)
    }
}

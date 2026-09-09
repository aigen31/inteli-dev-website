//! HTTP-клиент к OpenViking server.
//!
//! Реализует минимальный набор операций, нужных сайту: семантический поиск,
//! чтение/запись URI и health-check. Формат ответов разбирается максимально
//! терпимо, т.к. конкретный сервер может отдавать разные обёртки.

use std::time::Duration;

use serde::Deserialize;

use crate::error::{AppError, AppResult};

/// Результат семантического поиска по памяти.
#[derive(Debug, Clone, Deserialize)]
pub struct MemoryResult {
    pub uri: String,
    pub content: String,
    #[serde(default)]
    pub score: f64,
}

/// Клиент OpenViking (обёртка над HTTP API).
#[derive(Debug, Clone)]
pub struct OpenVikingClient {
    base_url: String,
    client: reqwest::Client,
}

impl OpenVikingClient {
    /// Создаёт клиент с таймаутами из конфигурации.
    pub fn new(base_url: impl Into<String>, timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("reqwest client build cannot fail with valid config");

        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client,
        }
    }

    /// Проверка доступности сервера.
    pub async fn health_check(&self) -> AppResult<()> {
        let resp = self
            .client
            .get(format!("{}/health", self.base_url))
            .send()
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AppError::OpenViking(format!(
                "health check: HTTP {}",
                resp.status()
            )))
        }
    }

    /// Семантический поиск по всем ресурсам памяти.
    pub async fn semantic_search(&self, query: &str, limit: usize) -> AppResult<Vec<MemoryResult>> {
        let body = serde_json::json!({
            "query": query,
            "limit": limit,
            "min_score": 0.35,
        });

        let resp = self
            .client
            .post(format!("{}/api/find", self.base_url))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(AppError::OpenViking(format!("find: HTTP {status}: {text}")));
        }

        self.parse_results(&text)
    }

    /// Читает содержимое конкретного URI.
    pub async fn read_uri(&self, uri: &str) -> AppResult<String> {
        let resp = self
            .client
            .post(format!("{}/api/read", self.base_url))
            .json(&serde_json::json!({ "uris": [uri] }))
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(AppError::OpenViking(format!(
                "read {uri}: HTTP {status}: {text}"
            )));
        }

        // Ответ может быть строкой, JSON-массивом записей или { content: ... }.
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(s) = value.as_str() {
                return Ok(s.to_string());
            }
            if let Some(content) = value.get("content").and_then(|c| c.as_str()) {
                return Ok(content.to_string());
            }
            if let Some(arr) = value.as_array() {
                let joined: String = arr
                    .iter()
                    .filter_map(|v| {
                        v.as_str()
                            .or_else(|| v.get("content").and_then(|c| c.as_str()))
                            .map(|s| s.to_string())
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n");
                return Ok(joined);
            }
        }

        Ok(text)
    }

    /// Записывает содержимое в URI (только из админ-панели).
    pub async fn write_uri(&self, uri: &str, content: &str) -> AppResult<()> {
        let resp = self
            .client
            .post(format!("{}/api/write", self.base_url))
            .json(&serde_json::json!({ "uri": uri, "content": content }))
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            Err(AppError::OpenViking(format!(
                "write {uri}: HTTP {status}: {text}"
            )))
        }
    }

    /// Терпимый разбор результатов поиска: принимает массив напрямую,
    /// объект с ключом `results`/`memories`/`resources` или строку.
    fn parse_results(&self, text: &str) -> AppResult<Vec<MemoryResult>> {
        let value: serde_json::Value = serde_json::from_str(text)
            .map_err(|e| AppError::OpenViking(format!("find: invalid JSON: {e}")))?;

        let arr = value
            .as_array()
            .cloned()
            .or_else(|| value.get("results").and_then(|v| v.as_array()).cloned())
            .or_else(|| value.get("memories").and_then(|v| v.as_array()).cloned())
            .or_else(|| value.get("resources").and_then(|v| v.as_array()).cloned())
            .unwrap_or_default();

        let mut out = Vec::new();
        for item in arr {
            if let Ok(r) = serde_json::from_value::<MemoryResult>(item) {
                out.push(r);
            }
        }
        Ok(out)
    }
}

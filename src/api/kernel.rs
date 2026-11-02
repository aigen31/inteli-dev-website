//! Internal contract for the central Kiki assistant kernel.
//!
//! These endpoints are not public. They are called server-to-server with the
//! service token from `KERNEL_SERVICE_TOKEN`. The site remains the owner of
//! content, prices, status and lead data; the kernel only receives tool schemas
//! and tool results.

use axum::extract::{Path, Query, State};
use axum::http::header::{self, HeaderMap};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::api::ApiError;
use crate::error::AppError;
use crate::memory::content::SiteContent;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ProfileQuery {
    #[serde(default)]
    pub actor_id: String,
    #[serde(default)]
    pub actor_role: String,
}

#[derive(Debug, Deserialize)]
pub struct ToolCall {
    #[serde(default)]
    pub conversation_id: String,
    #[serde(default)]
    pub actor: Value,
    #[serde(default)]
    pub arguments: Value,
}

pub async fn profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(_query): Query<ProfileQuery>,
) -> Result<Json<Value>, ApiError> {
    authorize(&state, &headers)?;
    let content = SiteContent::get();
    let system = crate::llm::prompt::chatbot_system_prompt(&content.profile);

    Ok(Json(json!({
        "system": system,
        "tools": [{
            "type": "function",
            "function": {
                "name": "site_lookup",
                "description": "Найти факты сайта: услуги, цены, статус, кейсы, статьи и контакты. Используй для вопросов о том, что делает автор и сколько это стоит.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Вопрос или ключевые слова посетителя."}
                    },
                    "required": ["query"],
                    "additionalProperties": false
                }
            }
        }],
        "suggested_next": [
            {"text": "Какие услуги вы делаете?", "type": "lead_request"},
            {"text": "Сколько стоит?", "type": "lead_request"},
            {"text": "Оставить заявку", "type": "lead_request"}
        ]
    })))
}

pub async fn tool(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(call): Json<ToolCall>,
) -> Result<Json<Value>, ApiError> {
    authorize(&state, &headers)?;
    if name != "site_lookup" {
        return Err(AppError::NotFound(format!("unknown assistant tool: {name}")).into());
    }

    let query = call
        .arguments
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if query.is_empty() {
        return Err(AppError::Validation("site_lookup.query is required".into()).into());
    }

    let context = crate::mcp::context_for(&state.db, query).await;
    let text = if context.trim().is_empty() {
        "В публичной базе знаний нет подходящих фактов. Не выдумывай детали, предложи уточнить вопрос или оставить заявку.".to_string()
    } else {
        context
    };

    Ok(Json(json!({
        "result": { "text": text },
        "tool": "site_lookup",
        "conversation_id": call.conversation_id,
    })))
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let expected = state.config.kernel.service_token.trim();
    if expected.is_empty() {
        return Err(AppError::NotFound("internal assistant is disabled".into()).into());
    }
    let channel = headers
        .get("x-kernel-channel")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if channel != "site" {
        return Err(AppError::Unauthorized.into());
    }
    let provided = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .unwrap_or_default();
    if !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        return Err(AppError::Unauthorized.into());
    }
    Ok(())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (a, b) in left.iter().zip(right) {
        difference |= a ^ b;
    }
    difference == 0
}

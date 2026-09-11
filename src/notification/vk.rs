//! VK Messages API — отправка уведомлений о лидах.

use crate::error::{AppError, AppResult};
use crate::storage::lead::Lead;

/// Отправляет уведомление о лиде админу. Пропускает, если VK не настроен.
pub async fn send_lead_notification(
    lead: &Lead,
    token: &str,
    admin_user_id: u64,
    client: &reqwest::Client,
) -> AppResult<()> {
    if token.is_empty() || admin_user_id == 0 {
        tracing::debug!("VK not configured, skipping lead notification");
        return Ok(());
    }

    let message = build_notification_message(lead);
    // random_id нужен для дедупликации при ретраях.
    let random_id = uuid::Uuid::new_v4().as_u128() as i64;

    let resp = client
        .post("https://api.vk.com/method/messages.send")
        .query(&[("access_token", token), ("v", "5.131")])
        .json(&serde_json::json!({
            "user_id": admin_user_id,
            "message": message,
            "random_id": random_id,
        }))
        .send()
        .await?;

    let body = resp.text().await?;
    let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| AppError::VkError {
        code: 0,
        message: e.to_string(),
    })?;

    if let Some(err) = parsed.get("error") {
        return Err(AppError::VkError {
            code: err["error_code"].as_i64().unwrap_or(0),
            message: err["error_msg"]
                .as_str()
                .unwrap_or("Unknown VK error")
                .to_string(),
        });
    }

    tracing::info!("lead {} notification sent to VK", lead.id);
    Ok(())
}

fn build_notification_message(lead: &Lead) -> String {
    // VK ограничивает длину сообщения — обрезаем слишком длинные.
    let truncated: String = lead.message.chars().take(1024).collect();
    format!(
        "НОВАЯ ЗАЯВКА #{}\n\n\
         Имя: {}\n\
         Email: {}\n\
         Телефон: {}\n\
         Источник: {}\n\n\
         Сообщение:\n{}",
        lead.id,
        lead.name,
        lead.email.as_deref().unwrap_or("—"),
        lead.phone.as_deref().unwrap_or("—"),
        lead.source,
        truncated,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_truncates_long_text() {
        let lead = Lead {
            id: 1,
            name: "Иван".into(),
            email: None,
            phone: None,
            message: "x".repeat(2000),
            source: "chat".into(),
            status: "new".into(),
            created_at: "2026-09-07T10:00:00Z".into(),
            updated_at: "2026-09-07T10:00:00Z".into(),
        };
        let msg = build_notification_message(&lead);
        assert!(msg.chars().count() <= 1024 + 200);
        assert!(msg.contains("#1"));
    }
}

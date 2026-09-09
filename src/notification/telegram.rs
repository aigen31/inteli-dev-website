//! Telegram Bot API — отправка уведомлений о лидах.

use crate::error::{AppError, AppResult};
use crate::storage::lead::Lead;

/// Отправляет уведомление о лиде админу. Пропускает, если бот не настроен.
pub async fn send_lead_notification(
    lead: &Lead,
    chat_id: i64,
    bot_token: &str,
    client: &reqwest::Client,
) -> AppResult<()> {
    if bot_token.is_empty() || chat_id == 0 {
        tracing::debug!("Telegram not configured, skipping lead notification");
        return Ok(());
    }

    let text = build_notification_text(lead);
    let keyboard = build_lead_keyboard(lead.id);

    let url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");
    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "HTML",
            "reply_markup": keyboard,
        }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::TelegramError { status, body });
    }

    tracing::info!("lead {} notification sent to Telegram", lead.id);
    Ok(())
}

fn build_notification_text(lead: &Lead) -> String {
    format!(
        "<b>📩 НОВАЯ ЗАЯВКА #{id}</b>\n\n\
         Имя: <code>{name}</code>\n\
         Email: {email}\n\
         Телефон: {phone}\n\
         Источник: <b>{source}</b>\n\n\
         Сообщение:\n<code>{message}</code>\n\n\
         ───────────────\n⏰ {created_at}",
        id = lead.id,
        name = escape_html(&lead.name),
        email = lead.email.as_deref().unwrap_or("—"),
        phone = lead.phone.as_deref().unwrap_or("—"),
        source = escape_html(&lead.source),
        message = escape_html(&lead.message),
        created_at = lead.created_at,
    )
}

/// Inline-клавиатура для быстрого управления заявкой прямо из Telegram.
fn build_lead_keyboard(lead_id: i64) -> serde_json::Value {
    serde_json::json!({
        "inline_keyboard": [
            [
                { "text": "✅ Конвертировать", "callback_data": format!("lead:convert:{lead_id}") },
                { "text": "⏸ В обработку", "callback_data": format!("lead:processing:{lead_id}") }
            ]
        ]
    })
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_escaping_neutralizes_tags() {
        assert_eq!(escape_html("<b>&"), "&lt;b&gt;&amp;");
    }

    #[test]
    fn notification_text_contains_lead_fields() {
        let lead = Lead {
            id: 42,
            name: "Иван".into(),
            email: Some("i@example.com".into()),
            phone: None,
            message: "нужен SEO".into(),
            source: "form".into(),
            status: "new".into(),
            created_at: "2026-09-07T10:00:00Z".into(),
            updated_at: "2026-09-07T10:00:00Z".into(),
        };
        let text = build_notification_text(&lead);
        assert!(text.contains("#42"));
        assert!(text.contains("Иван"));
        assert!(text.contains("нужен SEO"));
    }
}

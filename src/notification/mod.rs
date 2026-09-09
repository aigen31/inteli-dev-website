//! Notification Layer — отправка уведомлений о лидах в Telegram + VK.
//!
//! Принцип надёжности (см. vk-telegram-bridge.md): лид всегда сохраняется в
//! SQLite; уведомления — best-effort. Оба канала отправляются параллельно
//! через `tokio::join!`, сбой одного канала не роняет запрос.

pub mod telegram;
pub mod vk;

use crate::storage::lead::Lead;

/// Ошибка частичной доставки: фиксирует, какой канал не сработал.
#[derive(Debug, thiserror::Error)]
pub enum PartialNotificationError {
    #[error("Telegram failed: {0}")]
    Telegram(String),
    #[error("VK failed: {0}")]
    Vk(String),
    #[error("both channels failed: TG={telegram}, VK={vk}")]
    Both { telegram: String, vk: String },
}

/// Сервис уведомлений.
#[derive(Debug, Clone)]
pub struct NotificationService {
    telegram_chat_id: i64,
    telegram_bot_token: String,
    vk_token: String,
    vk_admin_user_id: u64,
    client: reqwest::Client,
}

impl NotificationService {
    pub fn new(
        telegram_chat_id: i64,
        telegram_bot_token: String,
        vk_token: String,
        vk_admin_user_id: u64,
    ) -> Self {
        Self {
            telegram_chat_id,
            telegram_bot_token,
            vk_token,
            vk_admin_user_id,
            client: reqwest::Client::new(),
        }
    }

    /// Отправляет уведомление о лиде через оба канала параллельно.
    pub async fn notify_all(&self, lead: &Lead) -> Result<(), PartialNotificationError> {
        let (tg, vk) = tokio::join!(
            telegram::send_lead_notification(
                lead,
                self.telegram_chat_id,
                &self.telegram_bot_token,
                &self.client,
            ),
            vk::send_lead_notification(lead, &self.vk_token, self.vk_admin_user_id, &self.client),
        );

        match (tg, vk) {
            (Ok(_), Ok(_)) => {
                tracing::info!("lead {} notification: Telegram ✅ VK ✅", lead.id);
                Ok(())
            }
            (Ok(_), Err(e)) => {
                tracing::warn!("lead {} notification: Telegram ✅ VK ❌ ({e})", lead.id);
                Err(PartialNotificationError::Vk(e.to_string()))
            }
            (Err(e), Ok(_)) => {
                tracing::warn!("lead {} notification: Telegram ❌ ({e}) VK ✅", lead.id);
                Err(PartialNotificationError::Telegram(e.to_string()))
            }
            (Err(t), Err(v)) => {
                tracing::error!(
                    "lead {} notification: Telegram ❌ ({t}) VK ❌ ({v})",
                    lead.id
                );
                Err(PartialNotificationError::Both {
                    telegram: t.to_string(),
                    vk: v.to_string(),
                })
            }
        }
    }
}

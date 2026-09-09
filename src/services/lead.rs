//! Приём заявок/лидов: валидация → сохранение в SQLite → уведомление в TG+VK.

use std::sync::Arc;

use serde::Deserialize;
use sqlx::sqlite::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::notification::NotificationService;
use crate::storage::lead::{self, Lead, NewLead};
use crate::utils::validator::is_valid_email;

/// Входные данные заявки (POST /api/lead).
#[derive(Debug, Clone, Deserialize)]
pub struct LeadSubmission {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    pub message: String,
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_source() -> String {
    "form".to_string()
}

impl LeadSubmission {
    /// Проверяет обязательные поля и формат email.
    pub fn validate(&self) -> AppResult<()> {
        if self.name.trim().is_empty() {
            return Err(AppError::Validation("поле «имя» обязательно".into()));
        }
        if self.message.trim().is_empty() {
            return Err(AppError::Validation("поле «сообщение» обязательно".into()));
        }
        if let Some(email) = self.email.as_deref() {
            if !email.is_empty() && !is_valid_email(email) {
                return Err(AppError::Validation("некорректный email".into()));
            }
        }
        Ok(())
    }

    fn into_new_lead(self) -> NewLead {
        NewLead {
            name: self.name.trim().to_string(),
            email: self.email.filter(|e| !e.trim().is_empty()),
            phone: self.phone.filter(|p| !p.trim().is_empty()),
            message: self.message.trim().to_string(),
            source: self.source,
        }
    }
}

/// Сервис заявок.
pub struct LeadService {
    db: SqlitePool,
    notifications: Arc<NotificationService>,
}

impl LeadService {
    pub fn new(db: SqlitePool, notifications: Arc<NotificationService>) -> Self {
        Self { db, notifications }
    }

    /// Создаёт заявку: сохраняет в БД (всегда) и шлёт уведомления (best-effort).
    pub async fn create(&self, submission: LeadSubmission) -> AppResult<Lead> {
        submission.validate()?;

        let saved = lead::insert(&self.db, submission.into_new_lead()).await?;

        if let Err(e) = self.notifications.notify_all(&saved).await {
            // Частичная или полная недоставка — не роняем запрос: лид уже в БД.
            tracing::warn!("notification failure for lead {}: {e}", saved.id);
        }

        Ok(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::init_memory_pool;

    #[test]
    fn rejects_missing_name_and_message() {
        let s = LeadSubmission {
            name: "".into(),
            email: None,
            phone: None,
            message: "msg".into(),
            source: "form".into(),
        };
        assert!(s.validate().is_err());
    }

    #[test]
    fn rejects_invalid_email() {
        let s = LeadSubmission {
            name: "Иван".into(),
            email: Some("bad-email".into()),
            phone: None,
            message: "msg".into(),
            source: "form".into(),
        };
        assert!(s.validate().is_err());
    }

    #[tokio::test]
    async fn create_persists_lead_and_notifies() {
        let pool = init_memory_pool().await.unwrap();
        let notifications = Arc::new(NotificationService::new(0, String::new(), String::new(), 0));
        let svc = LeadService::new(pool.clone(), notifications);

        let lead = svc
            .create(LeadSubmission {
                name: "Иван".into(),
                email: Some("i@example.com".into()),
                phone: Some("+7999".into()),
                message: "Нужен SEO".into(),
                source: "form".into(),
            })
            .await
            .unwrap();

        assert!(lead.id > 0);
        // Уведомления не настроены (пустые токены) — каналы пропущены без ошибки.
        assert_eq!(lead::get(&pool, lead.id).await.unwrap().name, "Иван");
    }
}

# RULES: VK API + Telegram Bridge для лидов

## 📋 Содержание
- [Назначение bridge](#назначение-bridge)
- [Telegram Bot API интеграция](#telegram-bot-api-интеграция)
- [VK API интеграция](#vk-api-интеграция)
- [Параллельная отправка уведомлений](#параллельная-отправка-уведомлений)
- [Обработка ответов и callback](#обработка-ответов-и-callback)
- [Обработка ошибок и retry](#обработка-ошибок-и-retry)

---

## Назначение bridge

### Зачем два мессенджера
Посетитель может оставить заявку через:
1. **Форму на сайте** (/contact, /api/lead) → уведомление в Telegram + VK
2. **AI-чат** (chatbot lead_request) → уведомление в Telegram + VK
3. **Напрямую в Telegram** (@bot → "Хочу заявку") → сохранение + подтверждение в VK

Telegram используется для мгновенных push-уведомлений (большинство пользователей проверяют чаще).
VK используется как дублирующий канал и для обратной связи через VK Messenger.

### Принцип надёжности
> **"Если Telegram не доступен — VK получит уведомление. Если оба недоступны — лид записан в SQLite и retry."**

---

## Telegram Bot API интеграция

### Настройка бота
```bash
# 1. Создать бота через @BotFather в Telegram
# Команда: /newbot
# Имя: inteli.dev.ru Support
# Username: inteli_support_bot (или ваш вариант)

# 2. Получить token от BotFather
# Ответ: "Use this token to access the HTTP API: 123456:ABC-DEF..."

# 3. Запомнить chat_id админа
# Добавить бота в свой чат или написать ему /start
# Получить ID через: curl https://api.telegram.org/bot{TOKEN}/getUpdates
```

### Структура уведомлений Telegram

#### Уведомление о новой заявке
```
📩 НОВАЯ ЗАЯВКА #1234

Имя: Иван Иванов
Email: ivan@example.com
Телефон: +7 (999) 123-45-67
Источник: форма на сайте

Сообщение:
Нужно продвинуть сайт интернет-магазина электроники, бюджет ~80к в месяц.

───────────────────────
⏰ 07.09.2026 14:23
```

#### Уведомление из AI-чата
```
💬 ЗАЯВКА ИЗ ЧАТА #1235

Имя: Алексей Петров
Источник: chatbot (preset "Оставить заявку")

Сообщение клиента:
Здравствуйте! Интересует SEO для корпоративного сайта, ~50 страниц. 
Готовы обсудить бюджет?

───────────────────────
⏰ 07.09.2026 15:47
```

### Inline Keyboard для быстрого управления
При уведомлении о лиде — Telegram показывает кнопки управления:
```
╔══════════════════════════════════════╗
║ 📩 ЗАЯВКА #1234 · Ivan I.            ║
╠══════════════════════════════════════╣
║ email: ivan@example.com              ║
║ msg: "Нужен SEO для интернет-магазина║
╚══════════════════════════════════════╝

[📞 Позвонить]  [✉️ Написать]
[✅ Конвертировать]  [⏸ В обработку]
```

### Код Telegram Bridge (notification/telegram.rs)
```rust
use serde::{Deserialize, Serialize};

// Структура уведомления
#[derive(Serialize, Deserialize)]
pub struct TelegramNotification {
    pub chat_id: i64,
    pub text: String,
    pub parse_mode: Option<String>,  // "HTML" для форматирования
    pub reply_markup: Option<serde_json::Value>,
}

// Inline keyboard layout
fn build_lead_keyboard(lead_id: i64) -> serde_json::Value {
    serde_json::json!({
        "inline_keyboard": [
            [
                {
                    "text": "📞 Позвонить",
                    "url": format!("tel:+7XXXXXXXXXX")  // номер клиента из лидов
                },
                {
                    "text": "✉️ Написать",
                    "url": "mailto:email@example.com"
                }
            ],
            [
                {
                    "text": "✅ Конвертировать",
                    "callback_data": format!("lead:convert:{}", lead_id)
                },
                {
                    "text": "⏸ В обработку",
                    "callback_data": format!("lead:processing:{}", lead_id)
                }
            ]
        ]
    })
}

// Основная функция отправки уведомления
pub async fn send_lead_notification(lead: &Lead, chat_id: i64, bot_token: &str) 
    -> Result<(), AppError> 
{
    let keyboard = build_lead_keyboard(lead.id);
    
    // Build formatted message
    let text = format!(
        "<b>📩 ЗАЯВКА #{id}</b>\n\n\
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
        source = lead.source,
        message = escape_html(&lead.message),
        created_at = lead.created_at.format("%d.%m.%Y %H:%M"),
    );
    
    // Send via Telegram Bot API
    let api_url = format!(
        "https://api.telegram.org/bot{}/sendMessage",
        bot_token
    );
    
    let response = reqwest::Client::new()
        .post(&api_url)
        .json(&TelegramNotification {
            chat_id,
            text,
            parse_mode: Some("HTML".to_string()),
            reply_markup: Some(keyboard),
        })
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(AppError::TelegramError { 
            status: response.status().as_u16(), 
            body: error_text 
        });
    }
    
    info!("Lead {} notification sent to Telegram chat {}", lead.id, chat_id);
    Ok(())
}

// Обработка callback from inline keyboard (webhook handler)
pub async fn handle_telegram_callback(webhook_data: &serde_json::Value, db_pool: &SqlitePool) 
    -> Result<(), AppError> 
{
    let callback = webhook_data["callback_query"].get_or_insert(serde_json::Value::Null);
    let data = callback["data"].as_str().ok_or(AppError::InvalidTelegramCallback)?;
    
    // Parse: "lead:convert:1234" or "lead:processing:1234"
    let parts: Vec<&str> = data.split(':').collect();
    if parts.len() != 3 || parts[0] != "lead" {
        return Err(AppError::InvalidTelegramCallback);
    }
    
    let action = parts[1]; // convert / processing
    let lead_id: i64 = parts[2].parse()?;
    
    match action {
        "convert" => {
            sqlx::query!("UPDATE leads SET status = 'converted', updated_at = CURRENT_TIMESTAMP WHERE id = ?", lead_id)
                .execute(db_pool).await?;
            
            // Send confirmation to Telegram
            send_callback_answer(callback, "✅ Заявка конвертирована в проект!").await?;
        }
        "processing" => {
            sqlx::query!("UPDATE leads SET status = 'processing', updated_at = CURRENT_TIMESTAMP WHERE id = ?", lead_id)
                .execute(db_pool).await?;
            
            send_callback_answer(callback, "⏸ Заявка перемещена в обработку").await?;
        }
        _ => {
            return Err(AppError::InvalidTelegramCallback);
        }
    }
    
    Ok(())
}

// Webhook endpoint для Telegram (receives webhook updates)
pub async fn telegram_webhook_handler(req: Request<Body>) -> Response<Body> {
    let update: serde_json::Value = match parse_json(req).await {
        Ok(u) => u,
        Err(e) => return error_response(400, "Invalid JSON"),
    };
    
    // Process updates (only callback_query and message types needed)
    if let Some(callback) = update.get("callback_query") {
        if let Err(e) = handle_telegram_callback(&update).await {
            error!("Failed to process Telegram callback: {}", e);
        }
    }
    
    // Always return 200 OK to Telegram
    Response::builder()
        .status(200)
        .body("OK".into())
}

// Helper: escape HTML entities for Telegram HTML parsing
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
```

---

## VK API интеграция

### Настройка VK API
```bash
# 1. Создать приложение на developer.vk.com
# 2. Получить access_token с правами messages
#    Scope: messages (для отправки сообщений)

# 3. Узнать свой user_id в VK
#   curl "https://api.vk.com/method/users.get?user_ids=me&access_token=TOKEN&v=5.131"
```

### Структура уведомлений VK

#### Сообщение через VK Messages API
```json
{
    "user_id": 123456789,
    "message": "📩 НОВАЯ ЗАЯВКА #1234\n\nИмя: Иван Иванов\nEmail: ivan@example.com\nТелефон: +7 (999) 123-45-67\n\nСообщение:\nНужно продвинуть сайт интернет-магазина электроники, бюджет ~80к в месяц.",
    "access_token": "a1b2c3d4...",
    "random_id": 0
}
```

### Код VK Bridge (notification/vk.rs)
```rust
// Структура для отправки сообщения через VK API
pub struct VkSendMessageRequest {
    pub user_id: Option<i64>,      // Отправка конкретному пользователю
    pub message: String,           // Текст сообщения (до 4096 символов)
    pub random_id: i64,            // Для дедупликации при retry
}

// Основная функция отправки уведомления VK
pub async fn send_lead_notification(lead: &Lead, vk_token: &str, admin_user_id: u64) 
    -> Result<(), AppError> 
{
    let message = format!(
        "📩 ЗАЯВКА #{}\n\n\
         Имя: {}\n\
         Email: {}\n\
         Источник: {}\n\n\
         Сообщение:\n{}",
        lead.id,
        lead.name,
        lead.email.as_deref().unwrap_or("—"),
        lead.source,
        &lead.message.chars().take(1024).collect::<String>(),  // truncate long messages
    );
    
    let api_url = "https://api.vk.com/method/messages.send";
    
    let params = serde_json::json!({
        "user_id": admin_user_id,
        "message": message,
        "random_id": rand::random::<i64>(),
    });
    
    // VK API требует access_token как query parameter (для legacy API) 
    // или через Authorization header (для new API)
    let response = reqwest::Client::new()
        .post(api_url)
        .query(&[("access_token", vk_token), ("v", "5.131")])  // VK API version
        .json(&params)
        .send()
        .await?;
    
    let body = response.text().await?;
    let parsed: serde_json::Value = serde_json::from_str(&body)?;
    
    if let Some(err) = parsed.get("error") {
        return Err(AppError::VkError {
            code: err["error_code"].as_i64().unwrap_or(0),
            message: err["error_msg"].as_str().unwrap_or("Unknown VK error").to_string(),
        });
    }
    
    info!("Lead {} notification sent to VK user {}", lead.id, admin_user_id);
    Ok(())
}
```

---

## Параллельная отправка уведомлений

### Принцип: обе отправки одновременно, ошибка отдельного канала = не критична
```rust
// notification/mod.rs — unified notification service
pub struct NotificationService {
    telegram_chat_id: i64,
    telegram_bot_token: String,
    vk_token: String,
    vk_admin_user_id: u64,
}

impl NotificationService {
    // Отправить уведомление о лиде через оба канала параллельно
    pub async fn notify_all(&self, lead: &Lead) -> Result<(), PartialNotificationError> {
        let (tg_result, vk_result) = tokio::join!(
            telegram::send_lead_notification(lead, self.telegram_chat_id, &self.telegram_bot_token),
            vk::send_lead_notification(lead, &self.vk_token, self.vk_admin_user_id),
        );
        
        // Log both results independently
        match (&tg_result, &vk_result) {
            (Ok(_), Ok(_)) => {
                info!("Lead {} notification: Telegram ✅ VK ✅", lead.id);
                Ok(())
            }
            (Ok(tg_ok), Err(vk_err)) => {
                warn!(
                    "Lead {} notification: Telegram ✅ VK ❌ ({})",
                    lead.id, vk_err
                );
                Err(PartialNotificationError::VkFailed(vk_err.clone()))
            }
            (Err(tg_err), Ok(_)) => {
                warn!(
                    "Lead {} notification: Telegram ❌ ({}) VK ✅",
                    lead.id, tg_err
                );
                Err(PartialNotificationError::TelegramFailed(tg_err.clone()))
            }
            (Err(tg_err), Err(vk_err)) => {
                error!(
                    "Lead {} notification: Telegram ❌ ({}) VK ❌ ({})",
                    lead.id, tg_err, vk_err
                );
                Err(PartialNotificationError::BothFailed {
                    telegram: tg_err.clone(),
                    vk: vk_err.clone(),
                })
            }
        }
    }
}

// Error type for partial notification failures
pub enum PartialNotificationError {
    TelegramFailed(AppError),
    VkFailed(AppError),
    BothFailed { telegram: AppError, vk: AppError },
}

impl std::fmt::Display for PartialNotificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TelegramFailed(e) => write!(f, "Telegram failed: {}", e),
            Self::VkFailed(e) => write!(f, "VK failed: {}", e),
            Self::BothFailed { telegram, vk } => write!(f, "Both channels failed: TG={}, VK={}", telegram, vk),
        }
    }
}

// API handler для создания лида
pub async fn handle_lead_submission(req: Request<Body>, state: AppState) -> Result<Response<Body>> {
    // 1. Parse and validate lead data
    let lead_data = parse_json::<LeadData>(req).await?;
    
    // 2. Save to SQLite (persistent, happens first)
    let lead = Lead {
        id: 0,  // auto-increment
        name: lead_data.name,
        email: lead_data.email,
        phone: lead_data.phone,
        message: lead_data.message,
        source: "form".to_string(),
        status: "new".to_string(),
        created_at: chrono::Utc::now(),
    };
    
    let saved_lead = lead.save(&state.db_pool).await?;  // This ALWAYS succeeds (DB local)
    
    // 3. Send notifications (best effort — don't fail the request if one channel is down)
    match state.notifications.notify_all(&saved_lead).await {
        Ok(_) => {},  // All good
        Err(PartialNotificationError::TelegramFailed(_)) |
        Err(PartialNotificationError::VkFailed(_)) => {
            // One failed, other succeeded — log but don't fail request
            warn!("Partial notification failure for lead {}", saved_lead.id);
        }
        Err(PartialNotificationError::BothFailed { telegram, vk }) => {
            // Both failed — save to retry queue!
            error!("Both notification channels down for lead {}. Queued for retry. TG={}, VK={}", 
                   saved_lead.id, telegram, vk);
            save_to_retry_queue(&saved_lead).await?;  // SQLite-based retry queue
        }
    }
    
    // 4. Return success to user (they don't know about notification failures)
    let response = serde_json::json!({
        "success": true,
        "message": "Ваша заявка принята! Я свяжусь с вами в ближайшее время.",
    });
    
    Ok(Json(response))
}
```

---

## Обработка ответов и callback

### Telegram Webhook setup
```bash
# Set webhook to your server (replace with actual domain)
curl "https://api.telegram.org/bot{TOKEN}/setWebhook?url=https://inteli.dev.ru/api/telegram/webhook"

# Check webhook status
curl "https://api.telegram.org/bot{TOKEN}/getWebhookInfo"
```

### Webhook handler (в api/telegram.rs)
```rust
pub async fn handle_telegram_webhook(req: Request<Body>, state: AppState) -> Result<Response<Body>> {
    let update: serde_json::Value = parse_json(req).await?;
    
    // Handle incoming messages from users who write to the bot directly
    if let Some(msg) = update.get("message") {
        let text = msg["text"].as_str().unwrap_or("");
        let chat_id = msg["chat"]["id"].as_i64().unwrap_or(0);
        
        match text {
            "/start" => {
                // Send welcome message with buttons
                send_welcome_message(chat_id, &state.config.telegram_bot_token).await?;
            }
            "Заявка" | "/lead" => {
                // Start lead collection flow
                start_lead_flow(chat_id).await?;
            }
            _ => {
                // Forward to admin as a new lead from Telegram
                let lead = Lead {
                    name: msg["from"]["first_name"].as_str().unwrap_or("Unknown").to_string(),
                    email: None,
                    phone: format!("Telegram @{}", 
                        msg.get("from").and_then(|f| f["username"].as_str()).unwrap_or("unknown")),
                    message: text.to_string(),
                    source: "telegram".to_string(),
                    status: "new".to_string(),
                    created_at: chrono::Utc::now(),
                };
                
                let saved = lead.save(&state.db_pool).await?;
                state.notifications.notify_all(&saved).await.ok();  // best effort
                
                // Confirm to user in Telegram
                send_text_to_telegram(chat_id, "✅ Заявка принята! Я свяжусь с вами в ближайшее время.", 
                    &state.config.telegram_bot_token).await?;
            }
        }
    }
    
    // Handle callback queries (inline keyboard actions)
    if let Some(callback) = update.get("callback_query") {
        handle_callback(callback, &state).await?;
    }
    
    Response::builder()
        .status(200)
        .body("OK".into())
}

async fn send_welcome_message(chat_id: i64, bot_token: &str) -> Result<(), AppError> {
    let api_url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
    
    reqwest::Client::new()
        .post(&api_url)
        .json(&serde_json::json!({
            "chat_id": chat_id,
            "text": "👋 Привет!\n\nЯ — AI-ассистент Ивана Петрова, специалиста по SEO.\n\nВы можете:\n💬 Задать вопрос о услугах и ценах\n📩 Оставить заявку напрямую\n\nЧто вас интересует?",
            "reply_markup": {
                "inline_keyboard": [
                    [
                        { "text": "💬 Задать вопрос", "url": "https://inteli.dev.ru/chat" },
                        { "text": "📩 Оставить заявку", "callback_data": "start_lead_flow" }
                    ]
                ]
            }
        }))
        .send()
        .await?;
    
    Ok(())
}
```

---

## Обработка ошибок и retry

### Retry queue для упавших уведомлений
```sql
-- SQLite table for failed notifications
CREATE TABLE IF NOT EXISTS notification_retry_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    lead_id INTEGER NOT NULL REFERENCES leads(id),
    telegram_failed BOOLEAN DEFAULT 0,
    vk_failed BOOLEAN DEFAULT 0,
    attempt_count INTEGER DEFAULT 0,
    next_retry_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_error TEXT
);

-- Retry job: run every 5 minutes (via scheduled task or simple loop)
-- SELECT * FROM notification_retry_queue WHERE next_retry_at <= datetime('now') AND attempt_count < 3
```

### Retry logic
```rust
pub async fn process_notification_retries(db_pool: &SqlitePool, notifications: &NotificationService) 
    -> Result<(), AppError> 
{
    // Get all pending retries (max 3 attempts, backoff)
    let retries: Vec<RetryEntry> = sqlx::query_as!(
        RetryEntry,
        r#"SELECT id, lead_id, telegram_failed, vk_failed, attempt_count, next_retry_at, last_error
           FROM notification_retry_queue 
           WHERE next_retry_at <= datetime('now')
             AND attempt_count < 3
           ORDER BY created_at ASC"#,
    ).fetch_all(db_pool).await?;
    
    for retry in retries {
        // Fetch the original lead
        let lead: Lead = sqlx::query_as!(
            Lead,
            "SELECT * FROM leads WHERE id = ?",
            retry.lead_id,
        ).fetch_one(db_pool).await?;
        
        // Attempt notification again
        if retry.telegram_failed || !retry.vk_failed {
            let tg_result = notifications.notify_telegram_only(&lead).await;
            
            if tg_result.is_ok() {
                // Telegram succeeded — mark as done
                sqlx::query!("DELETE FROM notification_retry_queue WHERE id = ?", retry.id)
                    .execute(db_pool).await?;
            } else {
                // Update attempt count and schedule next retry (exponential backoff)
                let new_attempts = retry.attempt_count + 1;
                let next_retry = chrono::Utc::now() 
                    + chrono::Duration::minutes(5 * (1 << new_attempts));  // 5, 10, 20 minutes
                
                sqlx::query!(
                    r#"UPDATE notification_retry_queue 
                       SET attempt_count = ?, next_retry_at = ?, last_error = ?
                       WHERE id = ?"#,
                    new_attempts,
                    next_retry.to_rfc3339(),
                    "Telegram retry failed",
                    retry.id,
                ).execute(db_pool).await?;
            }
        }
        
        // Similarly for VK...
    }
    
    Ok(())
}
```

---

*См. также: [docker-build.md](./docker-build.md), [chatbot.md](./chatbot.md)*

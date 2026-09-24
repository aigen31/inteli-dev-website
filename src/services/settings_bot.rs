//! Telegram-бот для управления настройками сайта.
//!
//! Отдельный бот (свой токен) для владельца: меняет состояние сайта прямо с
//! телефона — в первую очередь статус занятости. Тот бот, что присылает
//! заявки (`crate::notification`), остаётся «только на запись»: у него нет
//! прав что-либо менять.
//!
//! # Почему вебхук, а не long polling
//!
//! Сайт и так живёт за HTTPS (Traefik + Let's Encrypt), поэтому Telegram может
//! сам стучаться на `/api/settings-bot/webhook`. Long polling означал бы
//! постоянно висящий исходящий запрос к api.telegram.org и не дал бы ничего
//! взамен. Оплата — требование заголовка `X-Telegram-Bot-Api-Secret-Token`:
//! без него эндпоинт смог бы дёргать кто угодно и менять статус сайта.
//!
//! # Тестируемость
//!
//! Вся логика (разбор апдейта Telegram, разбор команды, переход состояния,
//! вёрстка ответа) — чистые функции. Сеть трогают только `send_message`,
//! `edit_message_text` и `answer_callback_query`, поэтому поведение бота
//! покрыто юнит-тестами без единого обращения к Telegram.

use serde_json::{json, Value};
use sqlx::sqlite::SqlitePool;

use crate::config::SettingsBotConfig;
use crate::error::{AppError, AppResult};
use crate::memory::content::Availability;
use crate::settings::{self, SiteSettings};

/// Максимальная длина свободного текста (`/slot`, `/when`).
const TEXT_FIELD_MAX_CHARS: usize = 120;
/// Максимальное число проектов в работе.
const MAX_PROJECTS: i64 = 99;
/// Заголовок, который Telegram присылает вместе с апдейтом.
pub const SECRET_HEADER: &str = "X-Telegram-Bot-Api-Secret-Token";

/// Разобранное входящее событие Telegram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incoming {
    pub user_id: i64,
    pub chat_id: i64,
    /// Текст сообщения (для обычных команд).
    pub text: Option<String>,
    /// `callback_data` inline-кнопки.
    pub callback_data: Option<String>,
    /// id callback-запроса — на него Telegram ждёт `answerCallbackQuery`.
    pub callback_query_id: Option<String>,
    /// id сообщения, которое нужно отредактировать вместо отправки нового.
    pub message_id: Option<i64>,
}

/// Команда владельца.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotCommand {
    ShowState,
    Help,
    SetStatus(String),
    SetProjects(i64),
    SetSlot(String),
    SetDate(String),
    /// Неизвестная команда или пустой текст.
    Unknown(String),
}

/// Результат применения команды к текущему состоянию.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applied {
    /// Состояние изменилось — нужно сохранить и опубликовать.
    Changed(Box<Availability>),
    /// Команда принята, но менять нечего (значение уже такое).
    Unchanged,
    /// Команда отклонена с объяснением для владельца.
    Rejected(String),
}

/// Клиент бота настроек.
#[derive(Debug, Clone)]
pub struct SettingsBot {
    http: reqwest::Client,
    token: String,
    admin_id: i64,
    webhook_secret: String,
    api_base: String,
}

impl SettingsBot {
    pub fn new(config: &SettingsBotConfig) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("reqwest client build cannot fail with valid config"),
            token: config.token.clone(),
            admin_id: config.admin_id,
            webhook_secret: config.webhook_secret.clone(),
            api_base: config.api_base.trim_end_matches('/').to_string(),
        }
    }

    /// Готов ли бот принимать команды.
    pub fn is_active(&self) -> bool {
        !self.token.is_empty() && self.admin_id != 0 && !self.webhook_secret.is_empty()
    }

    /// Проверяет секрет вебхука. Сравнение постоянного времени: иначе по
    /// времени ответа секрет можно было бы подобрать посимвольно.
    pub fn secret_matches(&self, presented: Option<&str>) -> bool {
        let Some(presented) = presented else {
            return false;
        };
        // Пустой секрет не должен совпадать с пустым заголовком.
        if self.webhook_secret.is_empty() {
            return false;
        }

        let expected = self.webhook_secret.as_bytes();
        let presented = presented.as_bytes();
        if expected.len() != presented.len() {
            return false;
        }
        expected
            .iter()
            .zip(presented)
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
    }

    /// Обрабатывает апдейт Telegram: отвечает владельцу, игнорирует чужих.
    ///
    /// Ошибки сети не всплывают наружу: Telegram всё равно получит 200,
    /// иначе он будет бесконечно повторять доставку одного и того же апдейта.
    pub async fn handle_update(&self, db: &SqlitePool, update: &Value) {
        let Some(incoming) = parse_update(update) else {
            return;
        };

        if incoming.user_id != self.admin_id {
            tracing::warn!(
                "settings bot: отказ пользователю {} (не владелец)",
                incoming.user_id
            );
            let text = format!(
                "Доступ запрещён.\n\nВаш Telegram ID: <code>{}</code>",
                incoming.user_id
            );
            self.reply(&incoming, &text, None).await;
            return;
        }

        // Нажатие inline-кнопки: сначала гасим «часики» у клиента, потому что
        // callback_query живёт ограниченное время и потом кнопка «зависает».
        if let Some(cb_id) = incoming.callback_query_id.as_deref() {
            let _ = self.answer_callback_query(cb_id, None).await;
        }

        let current = settings::load_or_default(db)
            .await
            .unwrap_or_else(|_| settings::defaults_from_content());

        let command = match incoming.callback_data.as_deref() {
            Some(data) => parse_callback(data),
            None => parse_command(incoming.text.as_deref().unwrap_or_default()),
        };

        // Справка не зависит от состояния и ничего не меняет.
        if command == BotCommand::Help {
            self.reply(&incoming, &render_help(), Some(status_keyboard()))
                .await;
            return;
        }

        let (text, keyboard) = match apply_command(&command, &current.availability) {
            Applied::Changed(next) => match settings::update_availability(db, *next).await {
                Ok(saved) => (render_state(&saved), Some(status_keyboard())),
                Err(e) => {
                    tracing::error!("settings bot: не удалось сохранить статус: {e}");
                    let text = format!(
                        "Не удалось сохранить: <code>{}</code>\n\n{}",
                        escape_html(&e.to_string()),
                        render_state(&current)
                    );
                    (text, Some(status_keyboard()))
                }
            },
            Applied::Unchanged => (
                format!(
                    "{}\n\n{}",
                    render_state(&current),
                    italic("Значение уже такое — ничего не изменилось.")
                ),
                Some(status_keyboard()),
            ),
            Applied::Rejected(reason) => (
                format!("{}\n\n{}", escape_html(&reason), render_state(&current)),
                Some(status_keyboard()),
            ),
        };

        self.reply(&incoming, &text, keyboard).await;
    }

    /// Отправляет новое сообщение либо редактирует исходное (для кнопок).
    async fn reply(&self, incoming: &Incoming, text: &str, keyboard: Option<Value>) {
        let result = match (incoming.message_id, incoming.callback_query_id.is_some()) {
            (Some(message_id), true) => {
                self.edit_message_text(incoming.chat_id, message_id, text, keyboard)
                    .await
            }
            _ => self.send_message(incoming.chat_id, text, keyboard).await,
        };

        if let Err(e) = result {
            tracing::warn!("settings bot: не удалось ответить: {e}");
        }
    }

    /// `sendMessage`.
    pub async fn send_message(
        &self,
        chat_id: i64,
        text: &str,
        keyboard: Option<Value>,
    ) -> AppResult<()> {
        let mut body = json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "HTML",
            "disable_web_page_preview": true,
        });
        if let Some(keyboard) = keyboard {
            body["reply_markup"] = keyboard;
        }
        self.call("sendMessage", &body).await
    }

    /// `editMessageText` — обновляет сообщение с кнопками вместо спама новыми.
    pub async fn edit_message_text(
        &self,
        chat_id: i64,
        message_id: i64,
        text: &str,
        keyboard: Option<Value>,
    ) -> AppResult<()> {
        let mut body = json!({
            "chat_id": chat_id,
            "message_id": message_id,
            "text": text,
            "parse_mode": "HTML",
            "disable_web_page_preview": true,
        });
        if let Some(keyboard) = keyboard {
            body["reply_markup"] = keyboard;
        }
        self.call("editMessageText", &body).await
    }

    /// `answerCallbackQuery` — убирает «часики» на нажатой кнопке.
    pub async fn answer_callback_query(
        &self,
        callback_query_id: &str,
        text: Option<&str>,
    ) -> AppResult<()> {
        let mut body = json!({ "callback_query_id": callback_query_id });
        if let Some(text) = text {
            body["text"] = json!(text);
        }
        self.call("answerCallbackQuery", &body).await
    }

    /// Вызов метода Bot API.
    pub async fn call(&self, method: &str, body: &Value) -> AppResult<()> {
        if self.token.is_empty() {
            return Err(AppError::TelegramError {
                status: 0,
                body: "токен бота настроек не задан".into(),
            });
        }

        let url = format!("{}/bot{}/{}", self.api_base, self.token, method);
        // Ошибку транспорта оборачиваем сами: `reqwest::Error` печатает URL,
        // а в нём токен бота (см. `AppError::telegram_transport`).
        let resp = self
            .http
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| AppError::telegram_transport(&e))?;
        let status = resp.status();

        if status.is_success() {
            return Ok(());
        }

        let raw = resp.text().await.unwrap_or_default();
        Err(AppError::TelegramError {
            status: status.as_u16(),
            body: truncate(&raw, 200),
        })
    }
}

/// Разбирает апдейт Telegram в [`Incoming`]. `None` — событие не про команды
/// (например, изменение в чате, которое нас не касается).
pub fn parse_update(update: &Value) -> Option<Incoming> {
    if let Some(cb) = update.get("callback_query").filter(|v| !v.is_null()) {
        return Some(Incoming {
            user_id: cb.pointer("/from/id").and_then(Value::as_i64)?,
            chat_id: cb
                .pointer("/message/chat/id")
                .and_then(Value::as_i64)
                .or_else(|| cb.pointer("/from/id").and_then(Value::as_i64))?,
            text: None,
            callback_data: cb.get("data").and_then(Value::as_str).map(str::to_string),
            callback_query_id: cb.get("id").and_then(Value::as_str).map(str::to_string),
            message_id: cb.pointer("/message/message_id").and_then(Value::as_i64),
        });
    }

    let message = update.get("message").filter(|v| !v.is_null())?;
    Some(Incoming {
        user_id: message.pointer("/from/id").and_then(Value::as_i64)?,
        chat_id: message.pointer("/chat/id").and_then(Value::as_i64)?,
        text: message.get("text").and_then(Value::as_str).map(str::to_string),
        callback_data: None,
        callback_query_id: None,
        message_id: message.get("message_id").and_then(Value::as_i64),
    })
}

/// Разбирает текст сообщения в команду.
///
/// Поддерживаются и `/команда`, и `/команда@botname` (так Telegram присылает
/// команды в группах), и `/команда аргумент`.
pub fn parse_command(text: &str) -> BotCommand {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return BotCommand::Unknown(String::new());
    }

    let (head, argument) = match trimmed.split_once(char::is_whitespace) {
        Some((head, rest)) => (head, rest.trim()),
        None => (trimmed, ""),
    };

    // Отбрасываем «@botname», если он есть.
    let head = head.split('@').next().unwrap_or(head);
    let head = head.to_ascii_lowercase();

    match head.as_str() {
        "/start" | "/status" | "/state" => BotCommand::ShowState,
        "/help" => BotCommand::Help,
        "/available" | "/free" => BotCommand::SetStatus("available".into()),
        "/busy" => BotCommand::SetStatus("busy".into()),
        "/full" => BotCommand::SetStatus("full".into()),
        "/projects" | "/proektov" => match argument.parse::<i64>() {
            Ok(n) => BotCommand::SetProjects(n),
            Err(_) => BotCommand::Unknown("Число проектов должно быть целым: /projects 2".into()),
        },
        "/slot" => {
            if argument.is_empty() {
                BotCommand::Unknown("Укажите слот: /slot с 1 октября".into())
            } else {
                BotCommand::SetSlot(argument.to_string())
            }
        }
        "/when" | "/date" => {
            if argument.is_empty() {
                BotCommand::Unknown("Укажите дату: /when 1 октября".into())
            } else {
                BotCommand::SetDate(argument.to_string())
            }
        }
        other => BotCommand::Unknown(format!("Неизвестная команда: {other}")),
    }
}

/// Разбирает `callback_data` inline-кнопки.
pub fn parse_callback(data: &str) -> BotCommand {
    match data {
        "set:available" => BotCommand::SetStatus("available".into()),
        "set:busy" => BotCommand::SetStatus("busy".into()),
        "set:full" => BotCommand::SetStatus("full".into()),
        "state:refresh" => BotCommand::ShowState,
        other => BotCommand::Unknown(format!("Неизвестная кнопка: {other}")),
    }
}

/// Применяет команду к состоянию. Чистая функция — вся валидация здесь.
pub fn apply_command(command: &BotCommand, current: &Availability) -> Applied {
    match command {
        // Показ состояния и справка не меняют данные: обрабатываются в
        // `render_*`, а здесь считаются «нечего менять».
        BotCommand::ShowState | BotCommand::Help => Applied::Unchanged,

        BotCommand::SetStatus(status) => {
            if !matches!(status.as_str(), "available" | "busy" | "full") {
                return Applied::Rejected(format!("Неизвестный статус: {status}"));
            }
            if current.status == *status {
                return Applied::Unchanged;
            }
            let mut next = current.clone();
            next.status = status.clone();
            Applied::Changed(Box::new(next))
        }

        BotCommand::SetProjects(n) => {
            if *n < 0 || *n > MAX_PROJECTS {
                return Applied::Rejected(format!(
                    "Число проектов должно быть от 0 до {MAX_PROJECTS}."
                ));
            }
            if current.current_projects as i64 == *n {
                return Applied::Unchanged;
            }
            let mut next = current.clone();
            next.current_projects = *n as u8;
            Applied::Changed(Box::new(next))
        }

        BotCommand::SetSlot(slot) => {
            let slot = slot.trim();
            if slot.is_empty() {
                return Applied::Rejected("Слот не может быть пустым.".into());
            }
            if slot.chars().count() > TEXT_FIELD_MAX_CHARS {
                return Applied::Rejected(format!(
                    "Слишком длинный текст: {} из {TEXT_FIELD_MAX_CHARS} символов.",
                    slot.chars().count()
                ));
            }
            if current.next_free_slot == slot {
                return Applied::Unchanged;
            }
            let mut next = current.clone();
            next.next_free_slot = slot.to_string();
            Applied::Changed(Box::new(next))
        }

        BotCommand::SetDate(date) => {
            let date = date.trim();
            if date.is_empty() {
                return Applied::Rejected("Дата не может быть пустой.".into());
            }
            if date.chars().count() > TEXT_FIELD_MAX_CHARS {
                return Applied::Rejected(format!(
                    "Слишком длинный текст: {} из {TEXT_FIELD_MAX_CHARS} символов.",
                    date.chars().count()
                ));
            }
            if current.availability_date == date {
                return Applied::Unchanged;
            }
            let mut next = current.clone();
            next.availability_date = date.to_string();
            Applied::Changed(Box::new(next))
        }

        BotCommand::Unknown(reason) => {
            if reason.is_empty() {
                Applied::Unchanged
            } else {
                Applied::Rejected(reason.clone())
            }
        }
    }
}

/// Текст сообщения с текущим состоянием сайта.
pub fn render_state(settings: &SiteSettings) -> String {
    let a = &settings.availability;
    format!(
        "<b>Настройки сайта</b>\n\n\
         Статус: <b>{status}</b>\n\
         Проектов в работе: <b>{projects}</b>\n\
         Ближайший слот: {slot}\n\
         Дата доступности: {date}\n\n\
         <i>Обновлено: {updated}</i>",
        status = crate::services::status::status_line(&a.status),
        projects = a.current_projects,
        slot = escape_html(&a.next_free_slot),
        date = escape_html(&a.availability_date),
        updated = format_timestamp(&settings.updated_at),
    )
}

/// Текст справки.
pub fn render_help() -> String {
    "<b>Бот настроек inteli-dev.ru</b>\n\n\
     Меняет состояние сайта. Кнопки ниже — быстрый способ, команды — точный.\n\n\
     /status — показать текущее состояние\n\
     /available — приём заявок открыт\n\
     /busy — приём заявок ограничен\n\
     /full — приём заявок закрыт\n\
     /projects 2 — сколько проектов в работе\n\
     /slot с 1 октября — ближайший свободный слот\n\
     /when 1 октября — дата доступности\n\n\
     Изменения сразу видны на сайте: бейдж в шапке, главная страница, \
     /api/status и ответы AI-чата."
        .to_string()
}

/// Inline-клавиатура с тремя статусами.
pub fn status_keyboard() -> Value {
    json!({
        "inline_keyboard": [
            [
                { "text": "Открыт", "callback_data": "set:available" },
                { "text": "Ограничен", "callback_data": "set:busy" },
                { "text": "Закрыт", "callback_data": "set:full" }
            ],
            [
                { "text": "Обновить", "callback_data": "state:refresh" }
            ]
        ]
    })
}

/// Экранирует текст для `parse_mode: HTML`.
pub fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn italic(text: &str) -> String {
    format!("<i>{}</i>", escape_html(text))
}

/// `2026-09-24T07:12:33+00:00` → `24.09.2026 07:12 UTC`.
fn format_timestamp(value: &str) -> String {
    match chrono::DateTime::parse_from_rfc3339(value) {
        Ok(dt) => {
            let utc = dt.with_timezone(&chrono::Utc);
            utc.format("%d.%m.%Y %H:%M UTC").to_string()
        }
        Err(_) => escape_html(value),
    }
}

fn truncate(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::content::fallback_content;

    fn base() -> Availability {
        fallback_content().availability
    }

    /// Конфиг бота с общими дефолтами (адрес API в тестах не используется —
    /// сеть тут не трогается вовсе).
    fn bot_config(token: &str, admin_id: i64, secret: &str) -> SettingsBotConfig {
        SettingsBotConfig {
            token: token.to_string(),
            admin_id,
            webhook_secret: secret.to_string(),
            ..Default::default()
        }
    }

    fn bot() -> SettingsBot {
        SettingsBot::new(&bot_config("123:abc", 330711327, "s3cret"))
    }

    // --- секрет вебхука ---

    #[test]
    fn secret_is_required_and_compared_exactly() {
        let bot = bot();
        assert!(bot.secret_matches(Some("s3cret")));
        assert!(!bot.secret_matches(Some("s3cre")));
        assert!(!bot.secret_matches(Some("s3cret ")));
        assert!(!bot.secret_matches(Some("")));
        assert!(!bot.secret_matches(None));
    }

    #[test]
    fn empty_secret_never_matches() {
        let bot = SettingsBot::new(&bot_config("123:abc", 1, ""));
        assert!(!bot.secret_matches(Some("")));
        assert!(!bot.is_active());
    }

    #[test]
    fn bot_needs_token_admin_and_secret() {
        assert!(bot().is_active());

        let no_admin = SettingsBot::new(&bot_config("123:abc", 0, "s"));
        assert!(!no_admin.is_active());

        let no_token = SettingsBot::new(&bot_config("", 5, "s"));
        assert!(!no_token.is_active());
    }

    // --- разбор апдейтов ---

    #[test]
    fn parses_plain_message() {
        let update = json!({
            "update_id": 1,
            "message": {
                "message_id": 10,
                "from": { "id": 330711327, "first_name": "Евгений" },
                "chat": { "id": 330711327, "type": "private" },
                "text": "/available"
            }
        });
        let incoming = parse_update(&update).expect("сообщение разобрано");
        assert_eq!(incoming.user_id, 330711327);
        assert_eq!(incoming.chat_id, 330711327);
        assert_eq!(incoming.text.as_deref(), Some("/available"));
        assert!(incoming.callback_query_id.is_none());
        assert_eq!(incoming.message_id, Some(10));
    }

    #[test]
    fn parses_callback_query() {
        let update = json!({
            "update_id": 2,
            "callback_query": {
                "id": "cb-1",
                "from": { "id": 330711327 },
                "message": { "message_id": 55, "chat": { "id": 330711327 } },
                "data": "set:busy"
            }
        });
        let incoming = parse_update(&update).expect("callback разобран");
        assert_eq!(incoming.callback_data.as_deref(), Some("set:busy"));
        assert_eq!(incoming.callback_query_id.as_deref(), Some("cb-1"));
        assert_eq!(incoming.message_id, Some(55));
        assert!(incoming.text.is_none());
    }

    #[test]
    fn ignores_updates_without_commands() {
        // Правка сообщения, join в чат и прочие события нас не касаются.
        assert!(parse_update(&json!({ "update_id": 3 })).is_none());
        assert!(
            parse_update(&json!({ "edited_message": { "text": "hi" } })).is_none()
        );
    }

    // --- разбор команд ---

    #[test]
    fn parses_status_commands() {
        assert_eq!(parse_command("/start"), BotCommand::ShowState);
        assert_eq!(parse_command("/status"), BotCommand::ShowState);
        assert_eq!(parse_command("  /STATUS  "), BotCommand::ShowState);
        assert_eq!(parse_command("/available"), BotCommand::SetStatus("available".into()));
        assert_eq!(parse_command("/busy"), BotCommand::SetStatus("busy".into()));
        assert_eq!(parse_command("/full"), BotCommand::SetStatus("full".into()));
    }

    #[test]
    fn strips_bot_suffix_from_command() {
        // В группах Telegram присылает /status@my_settings_bot.
        assert_eq!(parse_command("/status@inteli_settings_bot"), BotCommand::ShowState);
        assert_eq!(
            parse_command("/busy@inteli_settings_bot"),
            BotCommand::SetStatus("busy".into())
        );
    }

    #[test]
    fn parses_commands_with_arguments() {
        assert_eq!(parse_command("/projects 3"), BotCommand::SetProjects(3));
        assert_eq!(parse_command("/slot с 1 октября"), BotCommand::SetSlot("с 1 октября".into()));
        assert_eq!(parse_command("/when 1 октября"), BotCommand::SetDate("1 октября".into()));
        assert_eq!(parse_command("/help"), BotCommand::Help);
    }

    #[test]
    fn rejects_commands_with_bad_arguments() {
        assert!(matches!(parse_command("/projects много"), BotCommand::Unknown(_)));
        assert!(matches!(parse_command("/slot"), BotCommand::Unknown(_)));
        assert!(matches!(parse_command("/when"), BotCommand::Unknown(_)));
        assert!(matches!(parse_command("/nonsense"), BotCommand::Unknown(_)));
        assert!(matches!(parse_command("просто текст"), BotCommand::Unknown(_)));
    }

    // --- переходы состояния ---

    #[test]
    fn status_change_is_applied() {
        let current = base();
        match apply_command(&BotCommand::SetStatus("full".into()), &current) {
            Applied::Changed(next) => {
                assert_eq!(next.status, "full");
                // Остальные поля не должны пострадать.
                assert_eq!(next.current_projects, current.current_projects);
                assert_eq!(next.next_free_slot, current.next_free_slot);
            }
            other => panic!("ожидалось изменение, получено {other:?}"),
        }
    }

    #[test]
    fn repeated_status_is_not_a_change() {
        let current = base();
        assert_eq!(
            apply_command(&BotCommand::SetStatus(current.status.clone()), &current),
            Applied::Unchanged
        );
    }

    #[test]
    fn unknown_status_is_rejected() {
        let current = base();
        match apply_command(&BotCommand::SetStatus("sleeping".into()), &current) {
            Applied::Rejected(reason) => assert!(reason.contains("sleeping")),
            other => panic!("ожидался отказ, получено {other:?}"),
        }
    }

    #[test]
    fn projects_are_bounded() {
        let current = base();
        assert!(matches!(
            apply_command(&BotCommand::SetProjects(0), &current),
            Applied::Changed(_)
        ));
        assert!(matches!(
            apply_command(&BotCommand::SetProjects(99), &current),
            Applied::Changed(_)
        ));
        assert!(matches!(
            apply_command(&BotCommand::SetProjects(-1), &current),
            Applied::Rejected(_)
        ));
        assert!(matches!(
            apply_command(&BotCommand::SetProjects(100), &current),
            Applied::Rejected(_)
        ));
    }

    #[test]
    fn slot_and_date_are_trimmed_and_length_checked() {
        let current = base();

        match apply_command(&BotCommand::SetSlot("  с 1 октября  ".into()), &current) {
            Applied::Changed(next) => assert_eq!(next.next_free_slot, "с 1 октября"),
            other => panic!("ожидалось изменение, получено {other:?}"),
        }

        let long = "я".repeat(TEXT_FIELD_MAX_CHARS + 1);
        match apply_command(&BotCommand::SetDate(long.clone()), &current) {
            Applied::Rejected(reason) => assert!(reason.contains("121 из 120")),
            other => panic!("ожидался отказ, получено {other:?}"),
        }

        // Ровно на границе — принимается.
        let at_limit = "я".repeat(TEXT_FIELD_MAX_CHARS);
        assert!(matches!(
            apply_command(&BotCommand::SetDate(at_limit), &current),
            Applied::Changed(_)
        ));
    }

    #[test]
    fn show_state_and_help_change_nothing() {
        let current = base();
        assert_eq!(
            apply_command(&BotCommand::ShowState, &current),
            Applied::Unchanged
        );
        assert_eq!(apply_command(&BotCommand::Help, &current), Applied::Unchanged);
    }

    #[test]
    fn callbacks_map_to_the_same_commands_as_text() {
        assert_eq!(parse_callback("set:available"), BotCommand::SetStatus("available".into()));
        assert_eq!(parse_callback("set:busy"), BotCommand::SetStatus("busy".into()));
        assert_eq!(parse_callback("set:full"), BotCommand::SetStatus("full".into()));
        assert_eq!(parse_callback("state:refresh"), BotCommand::ShowState);
        assert!(matches!(parse_callback("drop:table"), BotCommand::Unknown(_)));
    }

    // --- вёрстка ---

    #[test]
    fn state_text_contains_every_field() {
        let settings = SiteSettings {
            availability: Availability {
                status: "busy".into(),
                current_projects: 3,
                next_free_slot: "с 1 октября".into(),
                availability_date: "2026-10-01".into(),
            },
            updated_at: "2026-09-24T07:12:33+00:00".into(),
        };
        let text = render_state(&settings);

        assert!(text.contains("Приём заявок: ограничен"));
        assert!(text.contains("<b>3</b>"));
        assert!(text.contains("с 1 октября"));
        assert!(text.contains("2026-10-01"));
        assert!(text.contains("24.09.2026 07:12 UTC"));
    }

    /// Бот настроек — четвёртая поверхность того же статуса. Он обязан говорить
    /// словами общего словаря, иначе владелец в Telegram и посетитель на сайте
    /// увидят разные формулировки.
    #[test]
    fn bot_vocabulary_matches_the_status_dictionary() {
        use crate::services::status::{presentation, SUBJECT};

        // Подлежащее в справке идёт со строчной буквы («приём заявок открыт»),
        // в словаре — с заглавной: сравниваем без регистра.
        let help = render_help().to_lowercase();
        assert!(
            help.contains(&SUBJECT.to_lowercase()),
            "справка потеряла подлежащее: {help}"
        );

        // Кнопки Telegram пишут состояние с заглавной, словарь — со строчной.
        let keyboard = status_keyboard().to_string().to_lowercase();
        for status in ["available", "busy", "full"] {
            let state = presentation(status).state;
            assert!(
                help.contains(state),
                "справка не знает состояния «{state}»: {help}"
            );
            assert!(
                keyboard.contains(state),
                "клавиатура не знает состояния «{state}»: {keyboard}"
            );
        }
    }

    #[test]
    fn state_text_escapes_html_in_user_values() {
        let settings = SiteSettings {
            availability: Availability {
                status: "available".into(),
                current_projects: 0,
                next_free_slot: "<script>alert(1)</script>".into(),
                availability_date: "2026-10-01".into(),
            },
            updated_at: "2026-09-24T07:12:33+00:00".into(),
        };
        let text = render_state(&settings);

        assert!(!text.contains("<script>"));
        assert!(text.contains("&lt;script&gt;"));
    }

    #[test]
    fn keyboard_has_all_statuses_and_refresh() {
        let keyboard = status_keyboard();
        let values = keyboard.to_string();
        assert!(values.contains("set:available"));
        assert!(values.contains("set:busy"));
        assert!(values.contains("set:full"));
        assert!(values.contains("state:refresh"));
    }

    #[test]
    fn help_mentions_every_command() {
        let help = render_help();
        for command in [
            "/status",
            "/available",
            "/busy",
            "/full",
            "/projects",
            "/slot",
            "/when",
        ] {
            assert!(help.contains(command), "в справке нет {command}");
        }
        // Единственные теги в справке — <b> и </b>: любой другой «<» сломал бы
        // parse_mode: HTML и Telegram отверг бы сообщение целиком.
        let tags = help.matches('<').count();
        assert_eq!(tags, help.matches("<b>").count() + help.matches("</b>").count());
    }
}

//! MCP-сервер сайта: один набор инструментов, два транспорта.
//!
//! # Зачем
//!
//! Ассистент в терминале знает только системный промпт и то, что найдётся в
//! семантическом поиске. Из-за этого на вопрос «делали ли вы такое» он отвечает
//! общими словами, хотя на сайте есть кейс с ровно этой задачей. MCP закрывает
//! разрыв: инструменты отдают факты сайта — услуги, кейсы, стек, цены, статус —
//! и модель цитирует их, а не сочиняет.
//!
//! # Почему не отдельный процесс
//!
//! Инструменты читают те же `SiteContent` и SQLite, что и сайт. Отдельный сервер
//! пришлось бы держать синхронным с основным (контент, прайс, статус), а это
//! второй источник правды. Здесь реестр один, а транспорта два:
//!
//! * **HTTP** `POST /mcp` — JSON-RPC 2.0 по MCP-спецификации для внешних
//!   агентов (Claude Desktop, Cursor, n8n).
//! * **В процессе** — [`context_for`] вызывает те же инструменты перед ответом
//!   ассистента, поэтому чат и внешний агент получают одинаковые факты.
//!
//! # Ограничения
//!
//! Инструменты только читают. Ни один из них не меняет прайс, статус или
//! заявки: MCP здесь — доступ к информации, а не админка. Запись живёт за
//! токеном админки и в MCP не выставляется.

use std::collections::BTreeSet;

use serde_json::{json, Value};

use crate::memory::content::{Project, Service, SiteContent};
use crate::state::AppState;
use crate::storage::price::{self, PriceItem};

/// Версия протокола, которую поддерживает сервер.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// Имя сервера в ответе `initialize`.
pub const SERVER_NAME: &str = "inteli-dev-site";

/// Описание инструмента для `tools/list`.
pub struct Tool {
    pub name: &'static str,
    pub description: &'static str,
    pub schema: fn() -> Value,
}

/// Реестр инструментов. Порядок — порядок показа агенту.
pub const TOOLS: &[Tool] = &[
    Tool {
        name: "site_map",
        description: "Разделы сайта inteli.dev: что где лежит и по какому адресу. \
                      Начинать знакомство с сайтом стоит отсюда.",
        schema: site_map_schema,
    },
    Tool {
        name: "search_site",
        description: "Поиск по услугам, кейсам и статьям сайта. Возвращает \
                      подходящие кейсы со стеком и результатом — это основной способ \
                      ответить на «делали ли вы подобное».",
        schema: search_schema,
    },
    Tool {
        name: "list_services",
        description: "Все услуги с описанием и ориентировочными ценами.",
        schema: empty_schema,
    },
    Tool {
        name: "find_cases",
        description: "Кейсы портфолио по технологии, задаче или отрасли. \
                      Например: «WordPress», «голосовые боты», «e-commerce».",
        schema: search_schema,
    },
    Tool {
        name: "find_articles",
        description: "Статьи блога сайта: разборы задач, решений и технологий. \
                      Использовать, когда спрашивают «писали ли вы об этом» или просят \
                      материал по теме.",
        schema: search_schema,
    },
    Tool {
        name: "get_prices",
        description: "Актуальный прайс-лист с сайта. Его ведёт владелец в админке, \
                      поэтому это единственный правильный источник цен.",
        schema: empty_schema,
    },
    Tool {
        name: "get_availability",
        description: "Текущая занятость: статус приёма заявок, проекты в работе, \
                      ближайший свободный слот.",
        schema: empty_schema,
    },
    Tool {
        name: "get_contacts",
        description: "Контакты автора: почта, Telegram, VK, телефон.",
        schema: empty_schema,
    },
];

fn empty_schema() -> Value {
    json!({ "type": "object", "properties": {}, "additionalProperties": false })
}

fn search_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Что искать: технология, задача, отрасль или слово из описания."
            }
        },
        "required": ["query"],
        "additionalProperties": false
    })
}

fn site_map_schema() -> Value {
    json!({ "type": "object", "properties": {}, "additionalProperties": false })
}

/// Результат вызова инструмента: текст для модели и признак ошибки.
pub struct ToolResult {
    pub text: String,
    pub is_error: bool,
}

impl ToolResult {
    fn ok(text: impl Into<String>) -> Self {
        Self { text: text.into(), is_error: false }
    }

    fn err(text: impl Into<String>) -> Self {
        Self { text: text.into(), is_error: true }
    }
}

/// Вызов инструмента по имени.
///
/// `arguments` — объект JSON из `tools/call`. Неизвестный инструмент — не
/// паника: внешний агент получает ошибку и список доступных имён.
pub async fn call(state: &AppState, name: &str, arguments: &Value) -> ToolResult {
    match name {
        "site_map" => ToolResult::ok(site_map()),
        "search_site" => search(state, arguments).await,
        "list_services" => ToolResult::ok(list_services()),
        "find_cases" => find_cases(arguments),
        "find_articles" => find_articles(&state.db, arguments).await,
        "get_prices" => get_prices(state).await,
        "get_availability" => ToolResult::ok(availability_text()),
        "get_contacts" => ToolResult::ok(contacts_text()),
        other => ToolResult::err(format!(
            "Неизвестный инструмент: {other}. Доступны: {}",
            TOOLS.iter().map(|t| t.name).collect::<Vec<_>>().join(", ")
        )),
    }
}

/// Карта сайта: разделы и что в них.
fn site_map() -> String {
    let content = SiteContent::get();
    let mut lines = vec![
        "# inteli.dev — карта сайта".to_string(),
        String::new(),
        "/ — главная: кто автор, направления, избранные кейсы, терминал с ИИ.".to_string(),
        "/services — услуги: направления, что входит, ориентиры по цене.".to_string(),
        format!(
            "/projects — {} кейсов с задачей, решением, стеком и результатом.",
            content.projects.len()
        ),
        "/blog — статьи: разборы задач и решений.".to_string(),
        "/chat — AI-ассистент по услугам, ценам и срокам.".to_string(),
        "/contact — форма заявки и прямые контакты.".to_string(),
        String::new(),
        "Кейсы:".to_string(),
    ];

    for project in &content.projects {
        lines.push(format!(
            "- {} — {} ({})",
            project.url_path(),
            project.title,
            project.niche
        ));
    }

    lines.join("\n")
}

/// Услуги с ценами из контента.
fn list_services() -> String {
    let content = SiteContent::get();
    let mut lines = vec!["# Услуги".to_string()];

    for service in &content.services {
        let price = match (&service.price_from, &service.price_to) {
            (Some(from), Some(to)) => format!("{from} – {to}"),
            (Some(from), None) => format!("от {from}"),
            (None, Some(to)) => format!("до {to}"),
            (None, None) => "по запросу".to_string(),
        };
        lines.push(format!(
            "\n## {}\n{}\nОриентир: {price}\nСтраница: /services",
            service.title, service.description
        ));
    }

    lines.join("\n")
}

/// Прайс из БД — то, что владелец правит в админке.
async fn get_prices(state: &AppState) -> ToolResult {
    match price::load_active(&state.db).await {
        Ok(items) if items.is_empty() => ToolResult::err(
            "Прайс-лист пуст. Цены не называй — предложи оставить заявку и посчитать \
             после описания задачи.",
        ),
        Ok(items) => ToolResult::ok(format_price_list(&items)),
        Err(e) => ToolResult::err(format!("Не удалось прочитать прайс: {e}")),
    }
}

fn format_price_list(items: &[PriceItem]) -> String {
    let mut lines = vec!["# Прайс-лист".to_string()];
    for item in items {
        lines.push(format!("- {}", item.line()));
    }
    lines.join("\n")
}

fn availability_text() -> String {
    let a = crate::settings::SiteSettings::availability();
    let mut lines = vec![crate::services::status::status_line(&a.status)];
    lines.extend(crate::services::status::facts(&a));
    lines.join("\n")
}

fn contacts_text() -> String {
    let c = &SiteContent::get().profile.contacts;
    format!(
        "Почта: {}\nTelegram: {}\nVK: {}\nТелефон: {}",
        c.email, c.telegram, c.vk, c.phone
    )
}

/// Поиск по всем текстам сайта: услуги, кейсы, статьи.
async fn search(_state: &AppState, arguments: &Value) -> ToolResult {
    let query = arguments
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();

    if query.is_empty() {
        return ToolResult::err("Нужен параметр query — что искать.");
    }

    let content = SiteContent::get();
    let tokens = tokenize(query);

    let mut services: Vec<(usize, &Service)> = content
        .services
        .iter()
        .map(|s| (score(&format!("{} {}", s.title, s.description), &tokens), s))
        .filter(|(score, _)| *score > 0)
        .collect();
    services.sort_by(|a, b| b.0.cmp(&a.0));

    let mut projects: Vec<(usize, &Project)> = content
        .projects
        .iter()
        .map(|p| (score_project(p, &tokens), p))
        .filter(|(score, _)| *score > 0)
        .collect();
    projects.sort_by(|a, b| b.0.cmp(&a.0));

    let mut lines = vec![format!("# Поиск: {query}")];

    // Сначала кейсы: на «делали ли вы такое» отвечает пример, а не описание услуги.
    if !projects.is_empty() {
        lines.push("\n## Кейсы".to_string());
        for (_, project) in projects.iter().take(3) {
            lines.push(project_block(project));
        }
    }

    if !services.is_empty() {
        lines.push("\n## Услуги".to_string());
        for (_, service) in services.iter().take(2) {
            lines.push(format!("- {} — {}", service.title, first_sentence(&service.description)));
        }
    }

    if projects.is_empty() && services.is_empty() {
        return ToolResult::ok(format!(
            "По запросу «{query}» на сайте ничего не нашлось. Не придумывай кейс: скажи, \
             что прямого примера нет, и предложи описать задачу."
        ));
    }

    ToolResult::ok(lines.join("\n"))
}

/// Кейсы по технологии/отрасли — отдельный инструмент, ответ подробнее.
fn find_cases(arguments: &Value) -> ToolResult {
    let query = arguments
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();

    if query.is_empty() {
        return ToolResult::err("Нужен параметр query — технология, задача или отрасль.");
    }

    let content = SiteContent::get();
    let tokens = tokenize(query);

    let mut matches: Vec<(usize, &Project)> = content
        .projects
        .iter()
        .map(|p| (score_project(p, &tokens), p))
        .filter(|(score, _)| *score > 0)
        .collect();
    matches.sort_by(|a, b| b.0.cmp(&a.0));

    if matches.is_empty() {
        return ToolResult::ok(format!(
            "Кейсов по «{query}» нет. Скажи об этом прямо и предложи посмотреть /projects."
        ));
    }

    let mut lines = vec![format!("# Кейсы по «{query}»")];
    for (_, project) in matches.iter().take(4) {
        lines.push(project_block(project));
    }
    ToolResult::ok(lines.join("\n"))
}

/// Статьи блога по тексту запроса: заголовок, теги и краткое описание.
async fn find_articles(db: &sqlx::SqlitePool, arguments: &Value) -> ToolResult {
    let query = arguments
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();

    let articles = match crate::storage::article::list_published_full(db).await {
        Ok(rows) => rows,
        Err(e) => return ToolResult::err(format!("Не удалось прочитать статьи: {e}")),
    };

    if articles.is_empty() {
        return ToolResult::ok("На сайте пока нет опубликованных статей.".to_string());
    }

    let tokens = tokenize(query);
    let mut scored: Vec<(usize, &crate::storage::article::ArticleRow)> = articles
        .iter()
        .map(|article| {
            let haystack = format!(
                "{} {} {}",
                article.title, article.summary, article.tags
            );
            let base = score(&haystack, &tokens);
            let title = score(&article.title, &tokens);
            (base + title * 2, article)
        })
        .filter(|(score, _)| *score > 0 || tokens.is_empty())
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));

    if scored.is_empty() {
        return ToolResult::ok(format!(
            "Статей по «{query}» нет. Скажи прямо и предложи посмотреть /blog."
        ));
    }

    let mut lines = vec!["# Статьи блога".to_string()];
    for (_, article) in scored.iter().take(3) {
        let published = article
            .published_at
            .as_deref()
            .map(|date| date.chars().take(10).collect::<String>())
            .unwrap_or_default();
        lines.push(format!(
            "\n- {} ({}) — /blog/{}\n  {}",
            article.title,
            published,
            article.slug,
            first_sentence(&article.summary)
        ));
    }

    ToolResult::ok(lines.join("\n"))
}

fn project_block(project: &Project) -> String {
    let stack = if project.stack.is_empty() {
        String::new()
    } else {
        format!("\nСтек: {}", project.stack.join(", "))
    };

    format!(
        "\n### {}\n{}\nЗадача: {}\nРезультат: {}{stack}\nСтраница: {}",
        project.title,
        project.summary,
        project.task,
        project.outcome,
        project.url_path()
    )
}

fn first_sentence(text: &str) -> String {
    text.split(". ")
        .next()
        .unwrap_or(text)
        .trim_end_matches('.')
        .to_string()
}

/// Нижний регистр и разбиение на слова: «AI-системы» → ai, системы.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| token.chars().count() >= 3)
        .map(ToString::to_string)
        .collect()
}

fn score(text: &str, tokens: &[String]) -> usize {
    let haystack = text.to_lowercase();
    tokens.iter().filter(|token| haystack.contains(token.as_str())).count()
}

fn score_project(project: &Project, tokens: &[String]) -> usize {
    let mut text = format!(
        "{} {} {} {} {} {}",
        project.title,
        project.niche,
        project.summary,
        project.task,
        project.solution.join(" "),
        project.result.join(" ")
    );
    text.push(' ');
    text.push_str(&project.stack.join(" "));
    if let Some(client) = &project.client {
        text.push(' ');
        text.push_str(client);
    }

    // Заголовок и стек весят больше описания: «WordPress» в стеке — точное
    // совпадение, а то же слово в абзаце может быть упоминанием мимоходом.
    let tokens_hit = score(&text, tokens);
    let stack_hit = score(&project.stack.join(" "), tokens);
    let title_hit = score(&project.title, tokens);
    tokens_hit + stack_hit * 2 + title_hit * 2
}

/// Контекст для ассистента: те же инструменты, что у внешнего агента.
///
/// Дешёвая эвристика вместо полноценного tool-calling: сначала ищем по тексту
/// вопроса, затем всегда добавляем прайс и карту сайта. Модель получает факты
/// до генерации, поэтому не выдумывает кейсы и цены.
pub async fn context_for(db: &sqlx::SqlitePool, question: &str) -> String {
    let q = question.to_lowercase();
    let mut parts: Vec<String> = Vec::new();

    // Кейсы подключаем только когда они действительно нашлись: иначе модель
    // получает пример «на всякий случай» и начинает рассказывать о проектах
    // вместо ответа.
    if !tokenize(question).is_empty() {
        let cases = find_cases(&json!({ "query": question }));
        if !cases.is_error && !cases.text.contains("Кейсов по") {
            parts.push(limit_chars(cases.text, 1200));
        }
    }

    // Цены и статус — только по своему поводу. Раньше прайс подмешивался в
    // каждый вопрос, и ответ на «можно ли обучить модель на наших данных»
    // начинался с перечня услуг.
    if has_any(&q, &["цен", "стоит", "стоимост", "бюджет", "прайс", "сколько", "дорого", "дешев"]) {
        if let Ok(items) = price::load_active(db).await {
            if !items.is_empty() {
                parts.push(limit_chars(format_price_list(&items), 900));
            }
        }
    }

    if has_any(&q, &["свобод", "занят", "срок", "когда", "очеред", "слот", "старт", "успе"]) {
        parts.push(availability_text());
    }

    // Статьи — по своему поводу: иначе модель пересказывает блог вместо ответа.
    if has_any(&q, &["стат", "блог", "читал", "написал", "разбор", "материал", "текст"]) {
        let articles = find_articles(db, &json!({ "query": question })).await;
        // В контекст идут только реальные статьи (у них есть адрес /blog/…):
        // «статей нет» — тоже ответ, но модели он не нужен.
        if !articles.is_error && articles.text.contains("/blog/") {
            parts.push(limit_chars(articles.text, 900));
        }
    }

    parts.join("\n\n")
}

/// Есть ли в вопросе хоть одно из слов-маркеров.
fn has_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

/// Обрезает контекст по границе строки: рвать фразу посередине хуже, чем
/// потерять последний факт.
fn limit_chars(text: String, max: usize) -> String {
    if text.chars().count() <= max {
        return text;
    }
    let mut out = String::new();
    for line in text.lines() {
        if out.chars().count() + line.chars().count() + 1 > max {
            break;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(line);
    }
    out
}

/// JSON-RPC 2.0 ответ на `tools/list`.
pub fn tools_list() -> Value {
    let tools: Vec<Value> = TOOLS
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": (tool.schema)(),
            })
        })
        .collect();

    json!({ "tools": tools })
}

/// Ответ на `initialize`.
pub fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION") },
        "instructions": "Инструменты сайта inteli.dev: кейсы, статьи блога, услуги, прайс, \
                         занятость, контакты. Кейс под задачу ищи через find_cases или \
                         search_site, материалы — через find_articles, цены бери только из \
                         get_prices."
    })
}

/// Обработка одного JSON-RPC запроса.
///
/// Возвращает `None` для уведомлений (у них нет ответа).
pub async fn handle_rpc(state: &AppState, request: &Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let params = request.get("params").cloned().unwrap_or(Value::Null);

    // Уведомления (`notifications/initialized`) ответа не требуют.
    if id.is_none() {
        return None;
    }
    let id = id.unwrap();

    let result = match method {
        "initialize" => Ok(initialize_result()),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(tools_list()),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            let outcome = call(state, name, &arguments).await;
            Ok(json!({
                "content": [{ "type": "text", "text": outcome.text }],
                "isError": outcome.is_error
            }))
        }
        other => Err((-32601, format!("Метод не поддерживается: {other}"))),
    };

    Some(match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": value }),
        Err((code, message)) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message }
        }),
    })
}

/// Уникальные имена инструментов — для теста реестра.
pub fn tool_names() -> BTreeSet<&'static str> {
    TOOLS.iter().map(|tool| tool.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_names_are_unique() {
        assert_eq!(tool_names().len(), TOOLS.len());
    }

    #[test]
    fn every_tool_has_description_and_schema() {
        for tool in TOOLS {
            assert!(!tool.description.is_empty(), "{} без описания", tool.name);
            let schema = (tool.schema)();
            assert_eq!(schema["type"], "object", "{}: схема не объект", tool.name);
        }
    }

    #[test]
    fn tokenizer_handles_cyrillic_and_punctuation() {
        let tokens = tokenize("Нужен AI-агент для WordPress!");
        assert!(tokens.contains(&"нужен".to_string()));
        assert!(tokens.contains(&"wordpress".to_string()));
        // Короткие слова отбрасываем: они дают ложные совпадения.
        assert!(!tokens.contains(&"ai".to_string()));
    }

    #[test]
    fn project_scoring_prefers_stack_match() {
        SiteContent::set_global(crate::memory::content::fallback_content());
        let content = SiteContent::get();
        let tokens = tokenize("wordpress");
        let mut best = (0usize, None);
        for project in &content.projects {
            let score = score_project(project, &tokens);
            if score > best.0 {
                best = (score, Some(project));
            }
        }
        let (score, project) = best;
        assert!(score > 0, "ни один кейс не совпал по WordPress");
        let project = project.unwrap();
        assert!(project.stack.iter().any(|s| s.to_lowercase().contains("wordpress")));
    }

    #[test]
    fn limit_chars_cuts_on_line_boundary() {
        let text = "первая строка\nвторая строка\nтретья строка".to_string();
        let cut = limit_chars(text, 30);
        assert!(cut.starts_with("первая строка"));
        assert!(!cut.contains("третья"));
    }

    #[test]
    fn intent_markers_match_russian_forms() {
        assert!(has_any("сколько это стоит?", &["стоит", "цен"]));
        assert!(has_any("когда сможете начать", &["когда", "срок"]));
        assert!(!has_any("можно ли обучить модель", &["цен", "срок"]));
    }

    #[test]
    fn price_list_formats_every_row() {
        let items = vec![PriceItem {
            slug: "audit".into(),
            title: "Аудит".into(),
            description: "2 дня".into(),
            price_from: "40 000".into(),
            price_to: "".into(),
            unit: "проект".into(),
            sort: 0,
            is_active: true,
            updated_at: String::new(),
        }];
        let text = format_price_list(&items);
        assert!(text.contains("Аудит (2 дня) — от 40 000 / проект"));
    }
}

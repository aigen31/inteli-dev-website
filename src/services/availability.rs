//! Живая занятость из CRM для виджета «Приём заявок».
//!
//! Сайт не хранит статус занятости: источник правды — CRM. Фоновая задача
//! раз в минуту забирает агрегаты (проекты, нагрузку по дням, ближайшее окно),
//! а SSR-страницы читают готовый снимок синхронно. Если CRM недоступна,
//! показываем последний удачный снимок, а до первого — значения из
//! [`SiteSettings`] как безопасный fallback.

use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

use chrono::{Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::memory::content::Availability;
use crate::services::status;

const REFRESH_EVERY: Duration = Duration::from_secs(60);
const CACHE_TTL: Duration = Duration::from_secs(120);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DayLoad {
    pub date: String,
    pub weekday: u32,
    pub load: u32,
    pub busy: bool,
    pub workday: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub status: String,
    pub active_projects: u32,
    pub busy_after: u32,
    pub next_free_slot: Option<String>,
    pub next_free_weekday: Option<u32>,
    pub days: Vec<DayLoad>,
    pub updated_at: String,
    #[serde(skip)]
    pub source: SnapshotSource,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SnapshotSource {
    #[default]
    Crm,
    Fallback,
}

#[derive(Debug, Deserialize)]
struct CrmPayload {
    status: String,
    active_projects: u32,
    #[serde(default = "default_busy_after")]
    busy_after: u32,
    #[serde(default)]
    next_free_slot: Option<String>,
    #[serde(default)]
    next_free_weekday: Option<u32>,
    #[serde(default)]
    days: Vec<DayLoad>,
    #[serde(default)]
    updated_at: String,
}

fn default_busy_after() -> u32 {
    4
}

static SNAPSHOT: OnceLock<RwLock<Option<(Instant, Snapshot)>>> = OnceLock::new();

fn cell() -> &'static RwLock<Option<(Instant, Snapshot)>> {
    SNAPSHOT.get_or_init(|| RwLock::new(None))
}

/// Запускает фоновое обновление снимка. Первый запрос — сразу, дальше раз в минуту.
pub fn spawn_refresh(base_url: String) {
    let base = base_url.trim_end_matches('/').to_string();

    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(4))
            .build();

        let client = match client {
            Ok(client) => client,
            Err(error) => {
                tracing::warn!("CRM availability: клиент не создан: {error}");
                return;
            }
        };

        loop {
            match fetch(&client, &base).await {
                Ok(snapshot) => {
                    let mut guard = cell().write().unwrap_or_else(|e| e.into_inner());
                    *guard = Some((Instant::now(), snapshot));
                }
                Err(error) => {
                    tracing::warn!("CRM availability: {error}");
                }
            }

            tokio::time::sleep(REFRESH_EVERY).await;
        }
    });
}

async fn fetch(client: &reqwest::Client, base: &str) -> Result<Snapshot, String> {
    let url = format!("{base}/api/public/availability");
    let response = client
        .get(&url)
        .header("accept", "application/json")
        .send()
        .await
        .map_err(|error| format!("запрос не удался: {error}"))?;

    if !response.status().is_success() {
        return Err(format!("CRM ответила {}", response.status()));
    }

    let payload: CrmPayload = response
        .json()
        .await
        .map_err(|error| format!("некорректный JSON: {error}"))?;

    Ok(Snapshot {
        status: payload.status,
        active_projects: payload.active_projects,
        busy_after: payload.busy_after,
        next_free_slot: payload.next_free_slot,
        next_free_weekday: payload.next_free_weekday,
        days: payload.days,
        updated_at: payload.updated_at,
        source: SnapshotSource::Crm,
    })
}

/// Текущий снимок. Просроченный кэш тоже лучше пустого: во время сбоя CRM
/// показываем последние известные данные, а не «закрыт».
pub fn snapshot() -> Snapshot {
    if let Ok(guard) = cell().read() {
        if let Some((at, snapshot)) = guard.as_ref() {
            if at.elapsed() <= CACHE_TTL {
                return snapshot.clone();
            }

            return snapshot.clone();
        }
    }

    fallback_snapshot()
}

/// Совместимое представление для чата, MCP, `/api/status` и старых мест.
pub fn current_availability() -> Availability {
    let snapshot = snapshot();
    let slot = slot_text(&snapshot);

    Availability {
        status: snapshot.status,
        current_projects: snapshot.active_projects.min(u8::MAX as u32) as u8,
        next_free_slot: slot.clone(),
        availability_date: slot,
    }
}

/// Факты для подсказки бейджа и текстовых ответов.
pub fn facts(snapshot: &Snapshot) -> Vec<String> {
    let busy_days = snapshot.days.iter().filter(|day| day.busy).count();
    let mut facts = vec![
        format!(
            "В работе {} {}",
            snapshot.active_projects,
            crate::limits::plural_ru(snapshot.active_projects, "проект", "проекта", "проектов")
        ),
        if snapshot.next_free_slot.is_some() || snapshot.source == SnapshotSource::Fallback {
            format!("Ближайшее окно: {}", slot_text(snapshot))
        } else {
            "Свободных окон в ближайшие 30 дней нет".to_string()
        },
    ];

    if !snapshot.days.is_empty() {
        facts.push(format!(
            "Занято {} из {} дней",
            busy_days,
            snapshot.days.len()
        ));
    }

    facts
}

pub fn aria_label(snapshot: &Snapshot) -> String {
    let mut parts = vec![
        status::status_line(&snapshot.status),
        status::presentation(&snapshot.status).summary.to_string(),
    ];
    parts.extend(facts(snapshot));
    parts.join(". ")
}

/// Слот для показа: у CRM — человеческая дата, у fallback — текст настроек.
pub fn slot_text(snapshot: &Snapshot) -> String {
    if snapshot.source == SnapshotSource::Fallback {
        let fallback = crate::settings::SiteSettings::availability().next_free_slot;
        if !fallback.trim().is_empty() {
            return fallback;
        }
    }

    slot_label(snapshot.next_free_slot.as_deref())
}

pub fn slot_label(iso: Option<&str>) -> String {
    let Some(date) = iso.and_then(parse_date) else {
        return "уточняется".to_string();
    };

    let today = Local::now().date_naive();

    if date == today {
        return "сегодня".to_string();
    }

    if date == today + chrono::Duration::days(1) {
        return "завтра".to_string();
    }

    format!("{} {}", date.day(), month_ru(date.month()))
}

pub fn weekday_short(weekday: u32) -> &'static str {
    match weekday {
        1 => "пн",
        2 => "вт",
        3 => "ср",
        4 => "чт",
        5 => "пт",
        6 => "сб",
        _ => "вс",
    }
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

fn month_ru(month: u32) -> &'static str {
    match month {
        1 => "января",
        2 => "февраля",
        3 => "марта",
        4 => "апреля",
        5 => "мая",
        6 => "июня",
        7 => "июля",
        8 => "августа",
        9 => "сентября",
        10 => "октября",
        11 => "ноября",
        _ => "декабря",
    }
}

fn fallback_snapshot() -> Snapshot {
    let a = crate::settings::SiteSettings::availability();

    Snapshot {
        status: a.status,
        active_projects: a.current_projects as u32,
        busy_after: 4,
        next_free_slot: None,
        next_free_weekday: None,
        days: Vec::new(),
        updated_at: crate::settings::SiteSettings::updated_at(),
        source: SnapshotSource::Fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_labels_are_human_readable() {
        assert_eq!(slot_label(None), "уточняется");
        assert_eq!(slot_label(Some("2026-11-27")), "27 ноября");
    }

    #[test]
    fn fallback_does_not_invent_busy_days() {
        use crate::memory::content::{builtin_content, SiteContent};
        SiteContent::set_global(builtin_content());

        let snapshot = fallback_snapshot();
        assert_eq!(snapshot.source, SnapshotSource::Fallback);
        assert!(snapshot.days.is_empty());
    }

    #[test]
    fn facts_mention_projects_and_window() {
        let snapshot = Snapshot {
            status: "available".into(),
            active_projects: 2,
            busy_after: 4,
            next_free_slot: Some("2026-11-27".into()),
            next_free_weekday: Some(5),
            days: vec![DayLoad {
                date: "2026-11-25".into(),
                weekday: 3,
                load: 5,
                busy: true,
                workday: true,
            }],
            updated_at: String::new(),
            source: SnapshotSource::Crm,
        };

        let facts = facts(&snapshot);
        assert!(facts[0].contains("2 проекта"), "{facts:?}");
        assert!(facts[1].contains("27 ноября"), "{facts:?}");
        assert!(facts[2].contains("1 из 1"), "{facts:?}");
    }
}

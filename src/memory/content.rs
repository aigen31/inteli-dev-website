//! Контент сайта: профиль автора, услуги, проекты, статус занятости.
//!
//! В идеале загружается из OpenViking. Если OpenViking недоступен или ещё не
//! настроен — используется [`fallback_content`] (встроенные значения, которые
//! владелец правит через админку в OpenViking).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::error::AppResult;
use crate::memory::OpenVikingClient;

/// Контакты автора.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contacts {
    pub email: String,
    pub telegram: String,
    pub vk: String,
    pub phone: String,
}

/// Профиль автора.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorProfile {
    pub name: String,
    pub title: String,
    pub experience_years: u8,
    pub projects_completed: u32,
    pub location: String,
    pub languages: Vec<String>,
    pub skills: Vec<String>,
    pub contacts: Contacts,
}

/// Услуга.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub slug: String,
    pub title: String,
    pub icon: String,
    pub description: String,
    pub price_from: Option<String>,
    pub price_to: Option<String>,
}

/// Кейс (проект) для портфолио.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub slug: String,
    pub title: String,
    pub niche: String,
    pub before: String,
    pub after: String,
    pub metric: String,
}

/// Статус занятости автора.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Availability {
    pub status: String, // "available" | "busy" | "full"
    pub current_projects: u8,
    pub next_free_slot: String,
    pub availability_date: String,
}

/// Полный контент сайта.
#[derive(Debug, Clone, Serialize)]
pub struct SiteContent {
    pub profile: AuthorProfile,
    pub services: Vec<Service>,
    pub projects: Vec<Project>,
    pub availability: Availability,
    pub status_updated_at: String,
}

static CONTENT: OnceLock<SiteContent> = OnceLock::new();

impl SiteContent {
    /// Сохраняет контент в глобальный кэш уровня приложения (L3).
    pub fn set_global(content: SiteContent) {
        let _ = CONTENT.set(content);
    }

    /// Возвращает глобальный контент (инициализируется при старте).
    pub fn get() -> &'static SiteContent {
        CONTENT.get().expect("SiteContent not initialized")
    }

    /// Загружает контент: пробует OpenViking, при ошибке — fallback.
    pub async fn load(client: &OpenVikingClient) -> SiteContent {
        match Self::load_from_openviking(client).await {
            Ok(content) => content,
            Err(e) => {
                tracing::warn!("OpenViking unavailable, using fallback content: {e}");
                fallback_content()
            }
        }
    }

    async fn load_from_openviking(client: &OpenVikingClient) -> AppResult<SiteContent> {
        client.health_check().await?;

        // Минимально-жизнеспособная загрузка: профиль из OpenViking, остальное — fallback.
        // Полная загрузка всех URI добавляется по мере наполнения OpenViking.
        let profile = client
            .read_uri("viking://user/inteli-dev/profile/main.md")
            .await
            .map(|md| parse_profile_markdown(&md))
            .unwrap_or_else(|_| fallback_content().profile);

        let mut content = fallback_content();
        content.profile = profile;
        Ok(content)
    }
}

/// Разбирает markdown-профиль в [`AuthorProfile`] (формат ключ-значение из правил).
fn parse_profile_markdown(md: &str) -> AuthorProfile {
    let mut profile = fallback_content().profile;

    for line in md.lines() {
        let line = line.trim();
        // Убираем маркер списка (`- ` или `* `), сохраняя markdown-bold `**Key**`.
        let line = line
            .strip_prefix("- ")
            .or_else(|| line.strip_prefix("* "))
            .unwrap_or(line);
        if let Some(rest) = line.strip_prefix("**Имя**") {
            profile.name = strip_markdown(rest);
        } else if let Some(rest) = line.strip_prefix("**Должность**") {
            profile.title = strip_markdown(rest);
        } else if let Some(rest) = line.strip_prefix("**Опыт**") {
            if let Some(years) = first_number(rest) {
                profile.experience_years = years as u8;
            }
        } else if let Some(rest) = line.strip_prefix("**Telegram**") {
            profile.contacts.telegram = strip_markdown(rest);
        } else if let Some(rest) = line.strip_prefix("**Email**") {
            profile.contacts.email = strip_markdown(rest);
        } else if let Some(rest) = line.strip_prefix("**VK**") {
            profile.contacts.vk = strip_markdown(rest);
        } else if let Some(rest) = line.strip_prefix("**Телефон**") {
            profile.contacts.phone = strip_markdown(rest);
        }
    }

    profile
}

fn strip_markdown(s: &str) -> String {
    s.trim()
        .trim_start_matches(':')
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string()
}

fn first_number(s: &str) -> Option<u32> {
    s.chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()
}

/// Встроенный fallback-контент (источник правды до настройки OpenViking).
///
/// Владелец меняет эти значения через админ-панель → OpenViking, не через код.
pub fn fallback_content() -> SiteContent {
    SiteContent {
        profile: AuthorProfile {
            name: "Иван Петров".to_string(),
            title: "Специалист по продвижению сайтов".to_string(),
            experience_years: 10,
            projects_completed: 200,
            location: "Россия, удалённая работа".to_string(),
            languages: vec!["Русский".to_string(), "English (B2)".to_string()],
            skills: vec![
                "SEO-аудит".to_string(),
                "Продвижение Яндекс/Google".to_string(),
                "Техническое SEO".to_string(),
                "Контент-стратегия".to_string(),
                "Аналитика и отчётность".to_string(),
            ],
            contacts: Contacts {
                email: "email@example.com".to_string(),
                telegram: "@username".to_string(),
                vk: "vk.com/username".to_string(),
                phone: "+7 (XXX) XXX-XX-XX".to_string(),
            },
        },
        services: vec![
            Service {
                slug: "seo-audit".into(),
                title: "SEO-аудит сайта".into(),
                icon: "🔍".into(),
                description:
                    "Технический аудит, анализ контента и конкурентов, отчёт с приоритетами.".into(),
                price_from: Some("15 000 ₽".into()),
                price_to: Some("50 000 ₽".into()),
            },
            Service {
                slug: "promotion".into(),
                title: "Продвижение в Яндекс/Google".into(),
                icon: "🚀".into(),
                description: "Комплексное продвижение: семантика, контент, ссылочный профиль."
                    .into(),
                price_from: Some("30 000 ₽/мес".into()),
                price_to: None,
            },
            Service {
                slug: "technical-seo".into(),
                title: "Техническая оптимизация".into(),
                icon: "⚙️".into(),
                description: "Скорость, Core Web Vitals, индексация, структура сайта.".into(),
                price_from: Some("20 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "content".into(),
                title: "Контент-стратегия".into(),
                icon: "✍️".into(),
                description: "Кластеры ключевых слов, план публикаций, оптимизация текстов.".into(),
                price_from: Some("25 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "analytics".into(),
                title: "Аналитика и отчётность".into(),
                icon: "📊".into(),
                description: "Настройка метрик, ежемесячные отчёты, рост трафика и заявок.".into(),
                price_from: Some("10 000 ₽/мес".into()),
                price_to: None,
            },
        ],
        projects: vec![
            Project {
                slug: "case-001".into(),
                title: "Интернет-магазин электроники".into(),
                niche: "E-commerce".into(),
                before: "100 визитов/день".into(),
                after: "800 визитов/день".into(),
                metric: "+700% трафика за 6 месяцев".into(),
            },
            Project {
                slug: "case-002".into(),
                title: "Корпоративный сайт застройщика".into(),
                niche: "Недвижимость".into(),
                before: "20 заявок/мес".into(),
                after: "120 заявок/мес".into(),
                metric: "+500% заявок за 4 месяца".into(),
            },
            Project {
                slug: "case-003".into(),
                title: "Блог о финансах".into(),
                niche: "Медиа".into(),
                before: "5 000 визитов/мес".into(),
                after: "40 000 визитов/мес".into(),
                metric: "+700% органического трафика".into(),
            },
        ],
        availability: Availability {
            status: "available".to_string(),
            current_projects: 5,
            next_free_slot: "начало ноября 2026".to_string(),
            availability_date: "2026-10-15".to_string(),
        },
        status_updated_at: "2026-09-07T10:00:00Z".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_content_is_complete() {
        let c = fallback_content();
        assert!(!c.profile.name.is_empty());
        assert_eq!(c.services.len(), 5);
        assert!(c.projects.len() >= 3);
        assert!(matches!(
            c.availability.status.as_str(),
            "available" | "busy" | "full"
        ));
    }

    #[test]
    fn profile_markdown_parses_name_and_contacts() {
        let md = r#"
# Профиль
- **Имя**: Иван Петров
- **Должность**: Специалист по SEO
- **Опыт**: 10 лет
- **Telegram**: @ivanov
- **Email**: ivan@example.com
"#;
        let p = parse_profile_markdown(md);
        assert_eq!(p.name, "Иван Петров");
        assert_eq!(p.experience_years, 10);
        assert_eq!(p.contacts.telegram, "@ivanov");
        assert_eq!(p.contacts.email, "ivan@example.com");
    }
}

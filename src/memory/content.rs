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
    pub icon_name: String,  // имя иконки Lucide (outline SVG)
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
            name: "Евгений Биль".to_string(),
            title: "Fullstack-разработчик и архитектор приватных AI-систем".to_string(),
            experience_years: 6,
            projects_completed: 100,
            location: "Красноярск, Россия · удалённо по РФ и СНГ".to_string(),
            languages: vec!["Русский".to_string(), "English (B2)".to_string()],
            skills: vec![
                "Приватные AI-системы".to_string(),
                "Локальный инференс (Qwen 3.6/3.8)".to_string(),
                "MCP-серверы и AI Skills".to_string(),
                "Голосовые ассистенты".to_string(),
                "Fullstack PHP + JS".to_string(),
                "Docker & DevOps".to_string(),
                "ComfyUI Mass Production".to_string(),
            ],
            contacts: Contacts {
                email: "contact@inteli-dev.ru".to_string(),
                telegram: "@AiGen31".to_string(),
                vk: "vk.com/aigen_31".to_string(),
                phone: "+7 (XXX) XXX-XX-XX".to_string(),
            },
        },
        services: vec![
            Service {
                slug: "ai-infrastructure".into(),
                title: "Приватные AI-системы".into(),
                icon_name: "brain-circuit".into(),
                description:
                    "Локальный инференс на собственном GPU-кластере (RTX 5080 + RTX 3060, 64GB RAM). Полная приватность данных, без облачных API-подписок. Qwen 3.6/3.8 (27B, 35B A3B) с глубокой настройкой под ваш домен.".into(),
                price_from: Some("100 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "ai-mcp".into(),
                title: "MCP-серверы и AI Skills".into(),
                icon_name: "plug-zap".into(),
                description:
                    "Собственные MCP-серверы для интеграции ИИ в ваш код. Голосовые ассистенты, автоматизация рабочих процессов, кастомные AI-навыки под бизнес-задачи.".into(),
                price_from: Some("80 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "voice-ai".into(),
                title: "Голосовые ИИ-боты".into(),
                icon_name: "mic".into(),
                description:
                    "Voice AI для холодных обзвонов, квалификации лидов, поддержки клиентов. LLM с динамическим ветвлением диалога, SIP/VoIP интеграция, TTS/STT настройка. Twin + LLM стек.".into(),
                price_from: Some("120 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "comfyui-workflows".into(),
                title: "ComfyUI Mass Production".into(),
                icon_name: "images".into(),
                description:
                    "Массовая генерация через ComfyUI — изображения, видео, дизайн-материалы. Автоматизированные пайплайны, настройка моделей, оптимизация под GPU.".into(),
                price_from: Some("50 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "fullstack-web".into(),
                title: "Fullstack PHP + JS разработка".into(),
                icon_name: "code-2".into(),
                description:
                    "Symfony, Laravel, React, Vue. REST API, мультиязычность, e-commerce, event-driven архитектура. Контейнеризация Docker + CI/CD. Проекты под ключ от проектирования до деплоя.".into(),
                price_from: Some("150 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "devops".into(),
                title: "DevOps & автоматизация серверов".into(),
                icon_name: "server-cog".into(),
                description:
                    "Docker-инфраструктура, Linux-серверы, Arch-based environments, CI/CD пайплайны. Автоматизация деплоя, мониторинг, масштабирование.".into(),
                price_from: Some("80 000 ₽".into()),
                price_to: None,
            },
        ],
        projects: vec![
            Project {
                slug: "voice-ai-real-estate".into(),
                title: "Голосовой ИИ-бот для продаж в недвижимости".into(),
                niche: "Voice AI · Недвижимость".into(),
                before: "Ручные обзвоны, высокая стоимость минуты".into(),
                after: "Автономная система квалификации лидов, 24/7 обзвон".into(),
                metric: "SIP/VoIP + LLM-ветвление · квалификация лидов 24/7".into(),
            },
            Project {
                slug: "ai-browser-agent".into(),
                title: "AI-агент для эмуляции реальных пользователей".into(),
                niche: "AI · Автоматизация".into(),
                before: "Ручные тесты, низкое покрытие сценариев".into(),
                after: "12+ модулей, ~3000 строк, anti-fingerprinting, human-like behavior".into(),
                metric: "Browser-use + Playwright · multi-worker orchestration ready".into(),
            },
            Project {
                slug: "coffee-b2b-platform".into(),
                title: "B2B/B2C платформа поставки кофе (Costa Rica)".into(),
                niche: "Laravel · E-commerce".into(),
                before: "Отсутствовала цифровая платформа поставок".into(),
                after: "Мультиязычная платформа на 8 языков, 2 платёжных шлюза, CI/CD".into(),
                metric: "Laravel 12 + Livewire 3 · event-driven ядро, 56 миграций".into(),
            },
            Project {
                slug: "pdd-pro-maintenance".into(),
                title: "Оптимизация pdd.pro — SQL, кэширование, бизнес-логика".into(),
                niche: "Symfony · Оптимизация".into(),
                before: "Медленные запросы, перегруженное кэширование".into(),
                after: "Оптимизированный SQL, переработанная бизнес-логика, рост скорости".into(),
                metric: "Symfony · индексы и кэш-слой, рост скорости отклика".into(),
            },
            Project {
                slug: "wordpress-devops-refactor".into(),
                title: "Рефакторинг WordPress + DevOps настройка сервера".into(),
                niche: "WordPress · DevOps".into(),
                before: "Медленный сайт, уязвимости, ручное администрирование".into(),
                after: "Ускоренный сайт, автоматизированный деплой, защита сервера".into(),
                metric: "WordPress + Docker · деплой и защита автоматизированы".into(),
            },
            Project {
                slug: "symfony-kimai-custom".into(),
                title: "Кастомизация Kimai и fullstack разработка на Symfony".into(),
                niche: "Symfony · SaaS".into(),
                before: "Стандартный функционал не покрывал бизнес-потребности".into(),
                after: "Полная кастомизация тайм-трекинга, REST API, fullstack интеграции".into(),
                metric: "Symfony + Vue/React · REST API, кастомный тайм-трекинг".into(),
            },
            Project {
                slug: "edu-platform-marketplace".into(),
                title: "Образовательная платформа и маркетплейс услуг".into(),
                niche: "WordPress · LMS · Маркетплейс".into(),
                before: "Нет единой платформы для обучения и продаж".into(),
                after: "WordPress + WooCommerce + LMS — обучение, маркетплейс, автоматизация".into(),
                metric: "WordPress + WooCommerce + LMS · платформа под ключ".into(),
            },
            Project {
                slug: "wordpress-woocommerce-plugins".into(),
                title: "Кастомные плагины WordPress и WooCommerce".into(),
                niche: "WordPress · Плагины".into(),
                before: "Готовые решения не решают уникальную бизнес-задачу".into(),
                after: "Кастомная бизнес-логика, автоматизация продаж, интеграции с внешними сервисами".into(),
                metric: "WordPress + WooCommerce · плагины полного цикла".into(),
            },
        ],
        availability: Availability {
            status: "available".to_string(),
            current_projects: 2,
            next_free_slot: "немедленно — есть свободные слоты".to_string(),
            availability_date: "2026-10-01".to_string(),
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
        assert_eq!(c.services.len(), 6);
        assert!(c.projects.len() >= 3);
        assert!(matches!(
            c.availability.status.as_str(),
            "available" | "busy" | "full"
        ));
    }

    /// Каждая услуга должна ссылаться на реально существующую иконку Lucide:
    /// иначе `LucideIcon` отрисует пустой `<svg>`.
    #[test]
    fn every_service_icon_exists_in_lucide_set() {
        use crate::ui::icon::ICON_PATHS;
        let c = fallback_content();
        for s in &c.services {
            assert!(
                ICON_PATHS.iter().any(|(k, _)| *k == s.icon_name),
                "иконка «{}» для услуги «{}» отсутствует в ICON_PATHS",
                s.icon_name,
                s.title
            );
        }
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

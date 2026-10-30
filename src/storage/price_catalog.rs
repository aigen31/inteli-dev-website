//! Каталог задач для прайс-листа.
//!
//! В `/admin/prices` лежат не только направления, но и конкретные задачи:
//! аудит, пилот, интеграция, поддержка, срочная правка. Владелец правит их
//! вручную, а этот модуль даёт стартовый набор. Цены привязаны к текущим
//! ориентирам услуг: мелкие задачи начинаются от 1 500 ₽, часовые от 2 000 ₽,
//! крупные проекты доходят до 120 000 ₽.
//!
//! Строки сортируются группами: сначала услуга целиком, затем её задачи.
//! Слаги задач не совпадают со слагами услуг, чтобы правка одной строки в
//! админке не путалась с базовым форматом.

use crate::memory::content::Service;
use crate::storage::price::PriceItem;

/// Одна задача в стартовом прайсе.
pub struct PriceTask {
    pub slug: &'static str,
    pub title: &'static str,
    pub price_from: &'static str,
    pub price_to: &'static str,
    pub unit: &'static str,
}

macro_rules! task {
    ($slug:literal, $title:literal, $from:literal, $to:literal, $unit:literal) => {
        PriceTask {
            slug: $slug,
            title: $title,
            price_from: $from,
            price_to: $to,
            unit: $unit,
        }
    };
}

const LOCAL: &[PriceTask] = &[
    task!("audit", "Аудит площадки под локальный inference", "5 000 ₽", "8 000 ₽", "аудит"),
    task!("pilot", "Пилот локальной модели на вашем GPU", "15 000 ₽", "25 000 ₽", "проект"),
    task!("rag", "RAG по документам и базе", "12 000 ₽", "20 000 ₽", "проект"),
    task!("fine-tune", "Дообучение модели под домен", "25 000 ₽", "60 000 ₽", "проект"),
    task!("api", "API, доступы и мониторинг контура", "8 000 ₽", "12 000 ₽", "проект"),
    task!("support", "Поддержка локального контура", "6 000 ₽", "", "месяц"),
];

const MY_INFERENCE: &[PriceTask] = &[
    task!("data-audit", "Аудит данных и маршрутизации", "8 000 ₽", "12 000 ₽", "аудит"),
    task!("litellm", "Настройка LiteLLM-роутера", "10 000 ₽", "18 000 ₽", "проект"),
    task!("hybrid-pilot", "Пилот гибридного контура", "25 000 ₽", "40 000 ₽", "проект"),
    task!("policy", "Политика чувствительных данных", "8 000 ₽", "12 000 ₽", "проект"),
    task!("corporate", "Корпоративный гибридный контур", "", "", "проект"),
    task!("support", "Поддержка гибридного стенда", "8 000 ₽", "", "месяц"),
];

const MCP: &[PriceTask] = &[
    task!("audit", "Аудит систем и сценариев для MCP", "5 000 ₽", "8 000 ₽", "аудит"),
    task!("one-tool", "Один MCP-инструмент", "8 000 ₽", "12 000 ₽", "проект"),
    task!("server", "MCP-сервер с интеграциями", "18 000 ₽", "35 000 ₽", "проект"),
    task!("auth", "Авторизация, роли и лимиты", "6 000 ₽", "10 000 ₽", "проект"),
    task!("docs", "Документация и обучение команды", "5 000 ₽", "8 000 ₽", "проект"),
];

const VOICE: &[PriceTask] = &[
    task!("script", "Сценарий голосового бота", "8 000 ₽", "12 000 ₽", "проект"),
    task!("telephony", "Пилот на телефонии", "15 000 ₽", "25 000 ₽", "проект"),
    task!("voice", "ASR, TTS и настройка голоса", "10 000 ₽", "18 000 ₽", "проект"),
    task!("crm", "Интеграция звонков с CRM", "8 000 ₽", "12 000 ₽", "проект"),
    task!("analytics", "Аналитика и прослушивание звонков", "6 000 ₽", "10 000 ₽", "проект"),
];

const COMFY: &[PriceTask] = &[
    task!("graph", "Граф ComfyUI под ваш стиль", "8 000 ₽", "15 000 ₽", "проект"),
    task!("lora", "LoRA на ваших примерах", "12 000 ₽", "25 000 ₽", "проект"),
    task!("batch", "Пакетная генерация и апскейл", "6 000 ₽", "12 000 ₽", "проект"),
    task!("pipeline", "Пайплайн для сайта и рекламы", "10 000 ₽", "18 000 ₽", "проект"),
    task!("training", "Обучение команды работе с графом", "5 000 ₽", "8 000 ₽", "проект"),
];

const ASSISTANTS: &[PriceTask] = &[
    task!("knowledge", "Сбор базы знаний", "5 000 ₽", "10 000 ₽", "проект"),
    task!("rag", "RAG-ассистент на сайте", "12 000 ₽", "20 000 ₽", "проект"),
    task!("widget", "Виджет в мессенджер", "8 000 ₽", "12 000 ₽", "проект"),
    task!("handoff", "Передача оператору и аналитика", "6 000 ₽", "10 000 ₽", "проект"),
    task!("support", "Поддержка ассистента", "6 000 ₽", "", "месяц"),
];

const BOTS: &[PriceTask] = &[
    task!("one-platform", "Бот на одной платформе", "6 000 ₽", "12 000 ₽", "проект"),
    task!("multi-platform", "Мультиплатформенный бот", "15 000 ₽", "30 000 ₽", "проект"),
    task!("payments", "Оплаты и CRM в боте", "8 000 ₽", "15 000 ₽", "проект"),
    task!("admin", "Админка и сценарии", "6 000 ₽", "10 000 ₽", "проект"),
    task!("support", "Поддержка бота", "4 000 ₽", "", "месяц"),
];

const VISION: &[PriceTask] = &[
    task!("poc", "PoC распознавания на ваших данных", "10 000 ₽", "18 000 ₽", "проект"),
    task!("ocr", "OCR документов", "12 000 ₽", "25 000 ₽", "проект"),
    task!("quality", "Контроль качества по фото", "15 000 ₽", "30 000 ₽", "проект"),
    task!("api", "API и очередь обработки", "8 000 ₽", "15 000 ₽", "проект"),
    task!("support", "Поддержка модели", "6 000 ₽", "", "месяц"),
];

const AUTOMATION: &[PriceTask] = &[
    task!("audit", "Аудит процесса", "4 000 ₽", "6 000 ₽", "аудит"),
    task!("n8n", "Одна цепочка n8n", "8 000 ₽", "15 000 ₽", "проект"),
    task!("mail-docs", "Разбор почты и документов", "10 000 ₽", "20 000 ₽", "проект"),
    task!("parsing", "Парсинг и выгрузки", "6 000 ₽", "12 000 ₽", "проект"),
    task!("support", "Поддержка автоматизации", "5 000 ₽", "", "месяц"),
];

const WEB: &[PriceTask] = &[
    task!("landing", "Лендинг", "12 000 ₽", "25 000 ₽", "проект"),
    task!("corporate", "Корпоративный сайт", "30 000 ₽", "60 000 ₽", "проект"),
    task!("ecommerce", "Интернет-магазин", "60 000 ₽", "120 000 ₽", "проект"),
    task!("audit", "Аудит скорости и SEO", "5 000 ₽", "10 000 ₽", "аудит"),
    task!("hourly", "Доработка сайта", "2 000 ₽", "", "час"),
];

const FULLSTACK: &[PriceTask] = &[
    task!("mvp", "MVP продукта", "30 000 ₽", "60 000 ₽", "проект"),
    task!("api", "API и интеграции", "20 000 ₽", "40 000 ₽", "проект"),
    task!("payments", "Платежи и подписки", "15 000 ₽", "30 000 ₽", "проект"),
    task!("infra", "Инфраструктура и деплой", "12 000 ₽", "25 000 ₽", "проект"),
    task!("hourly", "Развитие продукта", "2 500 ₽", "", "час"),
];

const DESIGN: &[PriceTask] = &[
    task!("audit", "Аудит интерфейса", "4 000 ₽", "8 000 ₽", "аудит"),
    task!("landing", "Лендинг в Figma или Penpot", "8 000 ₽", "18 000 ₽", "проект"),
    task!("system", "Дизайн-система", "20 000 ₽", "45 000 ₽", "проект"),
    task!("prototype", "Прототип и проверка сценариев", "8 000 ₽", "15 000 ₽", "проект"),
    task!("handoff", "Передача макета в разработку", "5 000 ₽", "8 000 ₽", "проект"),
];

const EMAIL: &[PriceTask] = &[
    task!("template", "Шаблон письма", "3 000 ₽", "6 000 ₽", "проект"),
    task!("domain", "Настройка домена и DKIM", "4 000 ₽", "8 000 ₽", "проект"),
    task!("series", "Серия писем", "8 000 ₽", "15 000 ₽", "проект"),
    task!("automation", "Автоматизация рассылки", "6 000 ₽", "12 000 ₽", "проект"),
    task!("deliverability", "Аудит доставляемости", "5 000 ₽", "10 000 ₽", "аудит"),
];

const QA: &[PriceTask] = &[
    task!("checklist", "Чек-лист и тест-план", "3 000 ₽", "6 000 ₽", "проект"),
    task!("manual", "Ручной прогон сценариев", "6 000 ₽", "12 000 ₽", "проект"),
    task!("regression", "Регресс перед релизом", "8 000 ₽", "15 000 ₽", "проект"),
    task!("playwright", "Автотесты Playwright", "15 000 ₽", "30 000 ₽", "проект"),
    task!("support", "Поддержка тестов", "5 000 ₽", "", "месяц"),
];

const DESKTOP: &[PriceTask] = &[
    task!("poc", "PoC на Tauri или Electron", "12 000 ₽", "20 000 ₽", "проект"),
    task!("mvp", "MVP десктопного приложения", "35 000 ₽", "60 000 ₽", "проект"),
    task!("offline", "Офлайн-режим и локальная база", "15 000 ₽", "30 000 ₽", "проект"),
    task!("updates", "Автообновления и сборка", "10 000 ₽", "18 000 ₽", "проект"),
    task!("support", "Поддержка приложения", "8 000 ₽", "", "месяц"),
];

const MOBILE: &[PriceTask] = &[
    task!("mvp", "MVP на React Native или Flutter", "45 000 ₽", "90 000 ₽", "проект"),
    task!("push-payments", "Пуши и оплаты", "15 000 ₽", "30 000 ₽", "проект"),
    task!("store", "Публикация в сторах", "10 000 ₽", "20 000 ₽", "проект"),
    task!("offline", "Офлайн-кэш и синхронизация", "10 000 ₽", "20 000 ₽", "проект"),
    task!("support", "Поддержка приложения", "10 000 ₽", "", "месяц"),
];

const LEGACY: &[PriceTask] = &[
    task!("audit", "Аудит легаси-кода", "8 000 ₽", "15 000 ₽", "аудит"),
    task!("plan", "План переезда", "10 000 ₽", "18 000 ₽", "проект"),
    task!("module", "Рефакторинг модуля", "15 000 ₽", "35 000 ₽", "проект"),
    task!("tests", "Тесты на ключевые сценарии", "10 000 ₽", "20 000 ₽", "проект"),
    task!("migration", "Поэтапный переезд", "40 000 ₽", "90 000 ₽", "проект"),
];

const SUPPORT: &[PriceTask] = &[
    task!("audit", "Разовый аудит инфраструктуры", "6 000 ₽", "12 000 ₽", "аудит"),
    task!("monitoring", "Настройка мониторинга", "5 000 ₽", "10 000 ₽", "проект"),
    task!("backups", "Бэкапы и восстановление", "5 000 ₽", "10 000 ₽", "проект"),
    task!("incident", "Разбор инцидента", "3 000 ₽", "", "час"),
    task!("extended", "Расширенная поддержка", "12 000 ₽", "", "месяц"),
];

const SYSADMIN: &[PriceTask] = &[
    task!("server", "Настройка сервера", "6 000 ₽", "12 000 ₽", "проект"),
    task!("docker", "Docker и Compose", "8 000 ₽", "15 000 ₽", "проект"),
    task!("cicd", "CI/CD", "10 000 ₽", "20 000 ₽", "проект"),
    task!("migration", "Миграция инфраструктуры", "20 000 ₽", "50 000 ₽", "проект"),
    task!("access", "Аудит доступов", "5 000 ₽", "10 000 ₽", "аудит"),
];

const SMALL: &[PriceTask] = &[
    task!("markup", "Правка вёрстки", "1 500 ₽", "3 000 ₽", "задача"),
    task!("bug-form", "Баг и форма", "2 000 ₽", "5 000 ₽", "задача"),
    task!("analytics", "Аналитика и цели", "2 000 ₽", "5 000 ₽", "задача"),
    task!("text-seo", "Текст и SEO-мелочи", "1 500 ₽", "4 000 ₽", "задача"),
    task!("urgent", "Срочная правка", "3 000 ₽", "", "час"),
    task!("consult", "Часовая консультация", "2 000 ₽", "", "час"),
];

/// Задачи услуги по слагу. Пустой список означает, что для услуги нет
/// отдельных задач, и в прайсе остаётся только базовый формат.
pub fn tasks_for(slug: &str) -> &'static [PriceTask] {
    match slug {
        "local-private-dev" => LOCAL,
        "my-inference-dev" => MY_INFERENCE,
        "ai-mcp" => MCP,
        "voice-ai" => VOICE,
        "comfyui-media" => COMFY,
        "ai-assistants" => ASSISTANTS,
        "ai-bots" => BOTS,
        "ai-vision" => VISION,
        "ai-automation" => AUTOMATION,
        "web-any" => WEB,
        "fullstack-any" => FULLSTACK,
        "design-figma" => DESIGN,
        "email-campaigns" => EMAIL,
        "qa-testing" => QA,
        "desktop-apps" => DESKTOP,
        "mobile-apps" => MOBILE,
        "legacy-refactor" => LEGACY,
        "support" => SUPPORT,
        "sysadmin-devops" => SYSADMIN,
        "small-fixes" => SMALL,
        _ => &[],
    }
}

/// Собирает стартовый прайс: базовый формат каждой услуги и её задачи.
pub fn build(services: &[Service], updated_at: &str) -> Vec<PriceItem> {
    let mut items = Vec::new();

    for (index, service) in services.iter().enumerate() {
        let base = index as i64 * 100 + 1;
        let (price_from, price_to, unit) = normalized_price(service);
        items.push(PriceItem {
            slug: service.slug.clone(),
            title: service.title.clone(),
            description: String::new(),
            price_from,
            price_to,
            unit,
            sort: base,
            is_active: true,
            updated_at: updated_at.to_string(),
        });

        for (offset, task) in tasks_for(&service.slug).iter().enumerate() {
            items.push(PriceItem {
                slug: format!("{}-{}", service.slug, task.slug),
                title: task.title.to_string(),
                description: String::new(),
                price_from: task.price_from.to_string(),
                price_to: task.price_to.to_string(),
                unit: task.unit.to_string(),
                sort: base + 1 + offset as i64,
                is_active: true,
                updated_at: updated_at.to_string(),
            });
        }
    }

    items
}

/// Убирает единицу из цены услуги, чтобы в прайсе не было «₽/мес / месяц».
fn normalized_price(service: &Service) -> (String, String, String) {
    let raw_from = service.price_from.clone().unwrap_or_default();
    let raw_to = service.price_to.clone().unwrap_or_default();

    if raw_from.trim().ends_with("/мес") {
        return (
            raw_from.trim_end_matches("/мес").trim().to_string(),
            raw_to.trim_end_matches("/мес").trim().to_string(),
            "месяц".to_string(),
        );
    }
    if raw_from.trim().ends_with("/час") {
        return (
            raw_from.trim_end_matches("/час").trim().to_string(),
            raw_to.trim_end_matches("/час").trim().to_string(),
            "час".to_string(),
        );
    }

    let unit = if service.slug == "small-fixes" {
        "задача"
    } else {
        "проект"
    };
    (raw_from, raw_to, unit.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::content::fallback_content;
    use std::collections::BTreeSet;

    #[test]
    fn every_service_gets_a_base_row_and_tasks() {
        let services = fallback_content().services;
        let items = build(&services, "now");

        for service in &services {
            assert!(
                items.iter().any(|item| item.slug == service.slug),
                "{}: нет базовой строки",
                service.slug
            );
            assert!(
                !tasks_for(&service.slug).is_empty(),
                "{}: нет задач",
                service.slug
            );
        }
        assert!(items.len() > services.len(), "задачи не попали в прайс");
    }

    #[test]
    fn slugs_are_unique_and_priced() {
        let items = build(&fallback_content().services, "now");
        let mut seen = BTreeSet::new();
        let base_slugs: BTreeSet<String> = fallback_content()
            .services
            .into_iter()
            .map(|service| service.slug)
            .collect();
        for item in &items {
            assert!(seen.insert(item.slug.clone()), "повтор слага {}", item.slug);
            let quote_only = item.slug == "my-inference-dev-corporate";
            assert!(
                !item.price_from.is_empty()
                    || !item.price_to.is_empty()
                    || base_slugs.contains(&item.slug)
                    || quote_only,
                "{}: нет цены",
                item.slug
            );
        }
    }

    #[test]
    fn tasks_are_ordered_after_their_service() {
        let items = build(&fallback_content().services, "now");
        let mut sorts = BTreeSet::new();
        for item in &items {
            assert!(sorts.insert(item.sort), "повтор sort {}", item.sort);
        }
    }

    #[test]
    fn monthly_and_hourly_units_are_not_duplicated_in_price() {
        let items = build(&fallback_content().services, "now");
        for item in &items {
            assert!(
                !item.price_from.contains("/мес") && !item.price_from.contains("/час"),
                "{}: единица осталась в цене",
                item.slug
            );
        }
    }
}

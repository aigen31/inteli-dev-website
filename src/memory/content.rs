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
    /// Категория для фильтра на `/services`: ai, web, infra, design, quality.
    /// Пустая строка — услуга попадёт в «Прочее».
    #[serde(default)]
    pub category: String,
    pub icon_name: String,  // имя иконки Lucide (outline SVG)
    pub description: String,
    pub price_from: Option<String>,
    pub price_to: Option<String>,
}

/// Кейс (проект) для портфолио.
///
/// Факты — из опубликованного портфолио автора, а не из головы: карточка в
/// списке и страница кейса показывают одно и то же, поэтому «кратко» и
/// «подробно» живут в одной структуре. `outcome` и `result` не дублируют друг
/// друга: первое — одна строка для карточки, второе — пункты для страницы.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub slug: String,
    pub title: String,
    /// Ниша: «Voice AI · Недвижимость».
    pub niche: String,
    /// Заказчик или продукт, если он назван публично («pdd.pro», «GS Coffee»).
    pub client: Option<String>,
    /// О чём проект — одно-два предложения для карточки.
    pub summary: String,
    /// Результат одной строкой для карточки.
    pub outcome: String,
    /// Что требовалось — из описания задачи.
    pub task: String,
    /// Что сделано, по пунктам.
    pub solution: Vec<String>,
    /// Что получилось, по пунктам.
    pub result: Vec<String>,
    /// Технологии, названные в описании. Пусто — блок «Стек» не показывается.
    pub stack: Vec<String>,
    /// Имя файла обложки в `/assets/projects/` (WebP, 1600 px по ширине).
    pub image: String,
    /// Публичный адрес проекта, если он есть.
    pub site_url: Option<String>,
}

impl Project {
    /// Адрес страницы кейса.
    pub fn url_path(&self) -> String {
        format!("/projects/{}", self.slug)
    }
}

/// Статус занятости автора.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Статус занятости **по умолчанию**: используется, пока в БД нет
    /// сохранённого значения. Живое состояние — в
    /// [`crate::settings::SiteSettings`]: его меняет Telegram-бот настроек.
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
                "Приватная разработка в локальном инференсе".to_string(),
                "Локальный инференс (Qwen 3.6/3.8)".to_string(),
                "MCP-серверы и AI Skills".to_string(),
                "Разработка под ключ".to_string(),
                "Docker & DevOps".to_string(),
                "Генерация изображений".to_string(),
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
                slug: "local-private-dev".into(),
                category: "ai".into(),
                title: "Приватная разработка в локальном инференсе".into(),
                icon_name: "brain-circuit".into(),
                description:
                    "Модели живут на вашем железе: подбираю и разворачиваю LLM на GPU, настраиваю дообучение под домен, собираю RAG по документам и закрываю доступ API. Данные не покидают вашу инфраструктуру, оплаты за каждый запрос нет.".into(),
                price_from: Some("25 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "ai-mcp".into(),
                category: "ai".into(),
                title: "MCP-серверы и AI Skills".into(),
                icon_name: "plug-zap".into(),
                description:
                    "MCP-серверы подключают модель к вашим системам: базе, CRM, файлам, API. Собираю инструменты и навыки, чтобы ассистент выполнял действия, а не только отвечал текстом.".into(),
                price_from: Some("10 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "voice-ai".into(),
                category: "ai".into(),
                title: "Голосовые ИИ-боты".into(),
                icon_name: "mic".into(),
                description:
                    "Звонки с живой речью: квалификация лидов, поддержка, напоминания. Ветвление диалога на LLM вместо жёсткого скрипта, SIP/VoIP, синтез и распознавание речи.".into(),
                price_from: Some("25 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "comfyui-media".into(),
                category: "ai".into(),
                title: "ComfyUI: генерация любых медиа с ИИ".into(),
                icon_name: "images".into(),
                description:
                    "Изображения, видео, анимация и звук в одном пайплайне: собираю графы ComfyUI под ваш стиль и объём, автоматизирую массовую генерацию, апскейл и подготовку материалов для сайта и рекламы.".into(),
                price_from: Some("8 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "ai-assistants".into(),
                category: "ai".into(),
                title: "ИИ-ассистенты".into(),
                icon_name: "message-circle".into(),
                description:
                    "Ассистент на сайте или в мессенджере: отвечает по вашей базе знаний, находит нужные документы, передаёт диалог человеку. Подключение к данным через RAG, чтобы ответы были по фактам, а не по памяти модели.".into(),
                price_from: Some("12 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "ai-bots".into(),
                category: "ai".into(),
                title: "Боты в Telegram, VK и WhatsApp".into(),
                icon_name: "message-circle".into(),
                description:
                    "Бот для заявок, оплат, уведомлений или поддержки — под конкретную платформу и её ограничения. Меню, сценарии, интеграции с сайтом и CRM, админка для ответов.".into(),
                price_from: Some("6 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "ai-vision".into(),
                category: "ai".into(),
                title: "Компьютерное зрение".into(),
                icon_name: "images".into(),
                description:
                    "Распознавание объектов и текста на фото, проверка качества по снимку, модерация изображений, OCR документов. Модель работает локально или в облаке — по требованиям к данным.".into(),
                price_from: Some("20 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "ai-automation".into(),
                category: "ai".into(),
                title: "Автоматизация процессов с ИИ".into(),
                icon_name: "settings".into(),
                description:
                    "Разбор почты и документов, парсинг сайтов, выгрузки и отчёты, цепочки n8n и скрипты. Модель подключается там, где нужен смысл, а не регулярка: классификация, извлечение полей, черновики ответов.".into(),
                price_from: Some("8 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "web-any".into(),
                category: "web".into(),
                title: "Веб-разработка на любом стеке".into(),
                icon_name: "code-2".into(),
                description:
                    "Лендинги, корпоративные сайты, магазины и личные кабинеты. React, Vue, Astro, PHP, Python — выбираю под задачу, а не под привычку; вёрстка, скорость, SEO и доступность в комплекте.".into(),
                price_from: Some("12 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "fullstack-any".into(),
                category: "web".into(),
                title: "Fullstack-разработка под ключ".into(),
                icon_name: "code-2".into(),
                description:
                    "От схемы данных до деплоя: API, фронтенд, фоновые задачи, платежи, интеграции, инфраструктура. Ведение проекта целиком, чтобы не собирать команду из пяти подрядчиков.".into(),
                price_from: Some("30 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "design-figma".into(),
                category: "design".into(),
                title: "Дизайн в Figma и Penpot".into(),
                icon_name: "images".into(),
                description:
                    "Интерфейсы, лендинги и дизайн-системы: макеты в Figma или Penpot, компоненты и токены, прототипы для проверки сценариев. Вёрстка после дизайна не расходится с макетом.".into(),
                price_from: Some("8 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "email-campaigns".into(),
                category: "design".into(),
                title: "Email-рассылки".into(),
                icon_name: "message-circle".into(),
                description:
                    "Письма, которые доходят и читаются: вёрстка под почтовые клиенты, домен и DKIM/SPF, сегменты, сценарии и автоматизация. Подключение к вашему сервису рассылок.".into(),
                price_from: Some("5 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "qa-testing".into(),
                category: "quality".into(),
                title: "QA и тестирование".into(),
                icon_name: "circle-check".into(),
                description:
                    "Ручное тестирование по сценариям и чек-листам, регресс перед релизом, автотесты на Playwright. Отчёт с шагами воспроизведения и приоритетом — чтобы правки не гадали.".into(),
                price_from: Some("6 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "desktop-apps".into(),
                category: "web".into(),
                title: "Десктопные приложения".into(),
                icon_name: "code-2".into(),
                description:
                    "Windows, macOS и Linux из одной кодовой базы на Tauri или Electron: работа с файлами, локальная база, офлайн-режим, обновления. Может нести локальную модель внутри.".into(),
                price_from: Some("35 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "mobile-apps".into(),
                category: "web".into(),
                title: "Мобильные приложения".into(),
                icon_name: "code-2".into(),
                description:
                    "iOS и Android на React Native или Flutter: навигация, пуш-уведомления, оплаты, офлайн-кэш и публикация в сторах. Один код на две платформы без потери нативного поведения.".into(),
                price_from: Some("45 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "legacy-refactor".into(),
                category: "infra".into(),
                title: "Рефакторинг легаси-проектов".into(),
                icon_name: "settings".into(),
                description:
                    "Разбираю старый код, нахожу узкие места и переношу проект на поддерживаемый стек постепенно, без остановки работы. Тесты на ключевые сценарии, чтобы рефакторинг не менял поведение.".into(),
                price_from: Some("15 000 ₽".into()),
                price_to: None,
            },
            Service {
                slug: "support".into(),
                category: "infra".into(),
                title: "Поддержка сервисов".into(),
                icon_name: "server-cog".into(),
                description:
                    "Обновления, бэкапы, мониторинг и разбор инцидентов по подписке. Слежу за доступностью и безопасностью, чтобы сайт не падал в самый неподходящий момент.".into(),
                price_from: Some("6 000 ₽/мес".into()),
                price_to: None,
            },
            Service {
                slug: "sysadmin-devops".into(),
                category: "infra".into(),
                title: "Системное администрирование и DevOps".into(),
                icon_name: "server-cog".into(),
                description:
                    "Linux-серверы, nginx, Docker, базы, бэкапы, CI/CD и мониторинг. Настройка с нуля или разбор текущей инфраструктуры — с документацией, чтобы доступы не жили в чьей-то голове.".into(),
                price_from: Some("2 000 ₽/час".into()),
                price_to: None,
            },
            Service {
                slug: "small-fixes".into(),
                category: "quality".into(),
                title: "Небольшие задачи и правки".into(),
                icon_name: "settings".into(),
                description:
                    "Поправить вёрстку, починить баг, обновить текст, подключить форму или аналитику. Беру задачи от 1 500 ₽ и считаю по часам — 2 000 ₽/час, если объём заранее не ясен.".into(),
                price_from: Some("1 500 ₽".into()),
                price_to: None,
            },
        ],
        // Кейсы — из опубликованного портфолио (freelance.ru/portfolio/user/aigen).
        // Порядок: сначала AI-разработка, затем веб-платформы и поддержка.
        projects: vec![
            Project {
                slug: "voice-ai-real-estate".into(),
                title: "Голосовой ИИ-бот для продаж в недвижимости".into(),
                niche: "Voice AI · Недвижимость".into(),
                client: None,
                summary: "Автоматический холодный обзвон базы по недвижимости: бот ведёт диалог, уточняет интерес и сам отсеивает нецелевых клиентов, чтобы менеджеры говорили только с тёплыми.".into(),
                outcome: "Автономная квалификация лидов: менеджерам уходят только тёплые контакты".into(),
                task: "Снизить стоимость контакта — минуты звонка — за счёт жёсткого отсеивания нецелевых клиентов, сохранив высокий уровень сервиса и вежливости.".into(),
                solution: vec![
                    "Ветвление диалога на LLM вместо жёсткого IVR-скрипта: бот определяет намерение — интерес, отказ, «не сейчас» — и подстраивает ответы.".into(),
                    "Платформа Twin выбрана как баланс между стоимостью трафика и качеством синтеза речи.".into(),
                    "SIP-транк для маршрутизации звонков на городские и мобильные номера РФ.".into(),
                    "Уведомления в Telegram: расшифровка разговора и статус лида — «готов к покупке», «ошибка связи», «отказ».".into(),
                    "Настройка TTS/STT: скорость и эмоции для делового, но не монотонного голоса; обработка обрывов связи и шума вокруг абонента.".into(),
                ],
                result: vec![
                    "Система готова к одновременному обзвону больших холодных баз.".into(),
                    "Скрипты продаж меняются без перезапуска бота с нуля.".into(),
                    "Паузы в ответах — не более одной секунды.".into(),
                ],
                stack: vec![
                    "Twin (Voice AI)".into(),
                    "SIP / VoIP".into(),
                    "Telegram Bot API".into(),
                    "LLM (GPT-4o-mini)".into(),
                    "Python".into(),
                    "Webhook".into(),
                ],
                image: "voice-ai-real-estate.webp".into(),
                site_url: None,
            },
            Project {
                slug: "ai-browser-agent".into(),
                title: "AI-агент для эмуляции реальных пользователей".into(),
                niche: "AI · Автоматизация браузера".into(),
                client: Some("agent-clicker".into()),
                summary: "Backend сервиса agent-clicker: агент на browser-use ведёт себя как живой посетитель сайта — ротация браузеров, устройств, IP и таймзон, изоляция сессий и анти-fingerprinting.".into(),
                outcome: "12+ модулей и ~3000 строк: mypy --strict, ruff и запуск одной командой".into(),
                task: "Спроектировать и реализовать backend-архитектуру сервиса agent-clicker: агент на browser-use эмулирует поведение реальных пользователей на целевых сайтах — с полной изоляцией сессий и антидетектом.".into(),
                solution: vec![
                    "Асинхронный сервис на Python 3.11+ и asyncio, интеграция browser-use (Agent + BrowserProfile).".into(),
                    "Двухуровневая конфигурация: статическая (env / pydantic-settings) и динамическая — через БД с TTL-кэшем и правкой из Admin Panel.".into(),
                    "Очередь задач на PostgreSQL без отдельного брокера: SELECT FOR UPDATE SKIP LOCKED и watchdog.".into(),
                    "Worker Pool с ограничением параллелизма, exponential backoff при повторах и hard timeout на agent.run().".into(),
                    "Профили браузера: случайный UA из 20+ реальных, согласованные viewport / dpr / locale / timezone, изоляция сессий через tmpdir.".into(),
                    "Приоритетный резолвинг прокси: задача → объявление → общий пул.".into(),
                    "WebSocket-стриминг логов в реальном времени (LogBroadcaster и кольцевой буфер).".into(),
                    "Docker: multi-stage сборка, healthcheck и graceful shutdown через tini.".into(),
                ],
                result: vec![
                    "Готовая к эксплуатации кодовая база: 12+ модулей, около 3000 строк.".into(),
                    "Все компоненты под type hints (mypy --strict), линтер ruff.".into(),
                    "Миграции через Alembic; запуск одной командой docker compose up.".into(),
                    "Архитектура готова к масштабированию: несколько воркеров, Redis для распределённых блокировок, метрики Prometheus.".into(),
                ],
                stack: vec![
                    "Python 3.11".into(),
                    "asyncio".into(),
                    "browser-use".into(),
                    "PostgreSQL".into(),
                    "FastAPI".into(),
                    "WebSocket".into(),
                    "Docker".into(),
                    "Alembic".into(),
                ],
                image: "ai-browser-agent.webp".into(),
                site_url: None,
            },
            Project {
                slug: "coffee-b2b-platform".into(),
                title: "B2B/B2C платформа поставки кофе".into(),
                niche: "Laravel · E-commerce".into(),
                client: Some("GS Coffee, Costa Rica".into()),
                summary: "Платформа для международного поставщика зелёного и обжаренного кофе: лоты, запросы котировок, B2C-магазин, проверка подлинности по серийным номерам и реферальная программа.".into(),
                outcome: "8 языков, два платёжных шлюза, 56 миграций — от схемы БД до CI/CD".into(),
                task: "Полный цикл разработки B2B/B2C e-commerce для международной компании: от проектирования архитектуры и схемы БД до контейнеризации и CI/CD.".into(),
                solution: vec![
                    "Схема БД на PostgreSQL: 56 миграций и ER-диаграмма.".into(),
                    "Управление лотами: зелёный и обжаренный кофе, публичные и приватные лоты, страницы с медиа.".into(),
                    "Запросы котировок с обменом сообщениями и воронкой статусов.".into(),
                    "B2C-магазин с корзиной, разными видами упаковки и помола.".into(),
                    "Заказы и инвойсы с генерацией PDF через DomPDF.".into(),
                    "Monobank Acquiring (UAH) и Stripe (USD): вебхуки, частичные оплаты, конвертация валют с кэшем курсов.".into(),
                    "Внутренний баланс пользователей с историей транзакций.".into(),
                    "Проверка подлинности по серийным номерам: QR-коды и уровни подозрительности.".into(),
                    "Реферальная программа с многоуровневыми комиссиями.".into(),
                    "8 языков через URL-префиксы и Spatie Translatable; админка на Filament 3 — 20+ ресурсов.".into(),
                    "Event-driven ядро: 11 событий, 16 слушателей и слой из 23 атомарных Actions.".into(),
                    "Docker-инфраструктура из 8 сервисов и CI/CD на GitHub Actions.".into(),
                ],
                result: vec![
                    "Мультиязычная платформа международного уровня, готовая к масштабированию.".into(),
                    "Изолированная Docker-инфраструктура с автоматическим TLS.".into(),
                    "Два платёжных провайдера с прозрачной конвертацией валют.".into(),
                    "Связка Livewire 3 + Alpine.js + Tailwind CSS 4.".into(),
                    "CI/CD с автоматическим тестированием и деплоем.".into(),
                ],
                stack: vec![
                    "Laravel 12".into(),
                    "Livewire 3".into(),
                    "Alpine.js".into(),
                    "Tailwind CSS 4".into(),
                    "PostgreSQL".into(),
                    "Filament 3".into(),
                    "Docker".into(),
                    "Traefik".into(),
                    "GitHub Actions".into(),
                ],
                image: "coffee-b2b-platform.webp".into(),
                site_url: Some("https://gscoffee.cr".into()),
            },
            Project {
                slug: "pdd-pro-maintenance".into(),
                title: "Доработка pdd.pro: SQL, кэш и рейтинг".into(),
                niche: "Агрегатор · Производительность".into(),
                client: Some("pdd.pro".into()),
                summary: "Агрегатор автошкол: убрали тяжёлые запросы и ошибку кэширования, починили расчёт средней оценки и фильтр школ — страницы стали открываться примерно на секунду быстрее.".into(),
                outcome: "Загрузка страниц быстрее примерно на секунду, нагрузка на БД ниже".into(),
                task: "Устранить узкие места в производительности и корректно реализовать бизнес-логику агрегатора: работа шла не только в интерфейсе, но и в логике и базе данных.".into(),
                solution: vec![
                    "Исправлена ошибка кэширования, убраны тяжёлые SQL-запросы.".into(),
                    "Оптимизированы SQL-запросы и работа с базой данных.".into(),
                    "Исправлен алгоритм подсчёта средней оценки — с учётом удаления записей автошкол из базы.".into(),
                    "Доработана логика фильтра автошкол под требования бизнеса.".into(),
                    "Улучшен UX агрегатора: поведение фильтров и элементов управления.".into(),
                ],
                result: vec![
                    "Загрузка страниц ускорена примерно на одну секунду.".into(),
                    "Снижена нагрузка на базу данных.".into(),
                    "Рейтинги и расчёты работают корректно.".into(),
                    "Улучшены фильтрация и взаимодействие с интерфейсом.".into(),
                ],
                stack: vec![
                    "SQL".into(),
                    "Кэширование".into(),
                ],
                image: "pdd-pro-maintenance.webp".into(),
                site_url: Some("https://pdd.pro".into()),
            },
            Project {
                slug: "wordpress-devops-refactor".into(),
                title: "Рефакторинг ormas.pro и перенос на VPS".into(),
                niche: "WordPress · DevOps".into(),
                client: Some("ORMAS".into()),
                summary: "Сайт центра детской психологии: рефакторинг темы, кэширование, перенос на VPS, Docker-инфраструктура и защита сервера после атаки.".into(),
                outcome: "Сайт ускорен, сервер изолирован в Docker, настроены UFW и Let's Encrypt".into(),
                task: "Комплексный рефакторинг сайта с упором на производительность, безопасность и подготовку к масштабированию — включая серверную инфраструктуру и перенос на VPS.".into(),
                solution: vec![
                    "Рефакторинг темы WordPress: оптимизация кода и упрощение структуры.".into(),
                    "Настройка кэширования: ускорение загрузки и снижение нагрузки на сервер.".into(),
                    "Перенос сайта на VPS без потери данных и простоев.".into(),
                    "Docker-инфраструктура: изоляция сервисов и удобный деплой.".into(),
                    "Настройка окружения сервера под проект.".into(),
                    "Безопасность: UFW, SSL через Let's Encrypt, проверка системы после атаки через ClamAV.".into(),
                    "Первичная SEO-оптимизация: robots.txt и подготовка под Google и Яндекс.".into(),
                ],
                result: vec![
                    "Сайт заметно ускорен.".into(),
                    "Повышены стабильность и отказоустойчивость.".into(),
                    "Улучшена безопасность после инцидента.".into(),
                    "Подготовлена база для SEO-продвижения.".into(),
                    "Проект готов к масштабированию и росту нагрузки.".into(),
                ],
                stack: vec![
                    "WordPress".into(),
                    "Docker".into(),
                    "VPS".into(),
                    "Nginx".into(),
                    "UFW".into(),
                    "Let's Encrypt".into(),
                    "ClamAV".into(),
                ],
                image: "wordpress-devops-refactor.webp".into(),
                site_url: Some("https://ormas.pro".into()),
            },
            Project {
                slug: "symfony-kimai-custom".into(),
                title: "Кастомизация Kimai на Symfony".into(),
                niche: "Symfony · Open-source".into(),
                client: Some("Kimai".into()),
                summary: "Форк тайм-трекера Kimai: интерфейс переработан по дизайн-макету до пикселя, бизнес-логика переписана под новые сценарии заказчика.".into(),
                outcome: "Pixel-perfect вёрстка по макету и новый UX поверх существующей архитектуры".into(),
                task: "Разработать кастомное решение на основе форка Kimai с полной переработкой интерфейса и бизнес-логики под требования заказчика.".into(),
                solution: vec![
                    "Форк и адаптация сервиса Kimai.".into(),
                    "Переработка фронтенда по дизайн-макету — pixel-perfect.".into(),
                    "Новый UX интерфейса.".into(),
                    "Доработка backend-логики под новые сценарии.".into(),
                    "Стек Symfony + Twig; оптимизация структуры кода и связей компонентов.".into(),
                ],
                result: vec![
                    "Полностью кастомизированный интерфейс.".into(),
                    "Улучшенный пользовательский опыт.".into(),
                    "Гибкая система под задачи бизнеса.".into(),
                    "Готовность к развитию и масштабированию.".into(),
                ],
                stack: vec![
                    "Symfony".into(),
                    "PHP".into(),
                    "Twig".into(),
                ],
                image: "symfony-kimai-custom.webp".into(),
                site_url: None,
            },
            Project {
                slug: "edu-platform-marketplace".into(),
                title: "Платформа курсов и услуг с LMS".into(),
                niche: "WordPress · LMS · Маркетплейс".into(),
                client: Some("beauty-inna.pro".into()),
                summary: "Онлайн-платформа косметологии, где услуги и курсы продаются в одной экосистеме: WooCommerce для магазина, Tutor LMS для обучения, мультиязычность и reverse proxy для доступности в разных странах.".into(),
                outcome: "Магазин и LMS в одном сайте, мультиязычность и стабильный доступ из ЕС".into(),
                task: "Разработать платформу для продажи услуг и курсов в сфере косметологии: магазин и онлайн-обучение в единой экосистеме, с мультиязычностью и доступностью в России, Украине и странах ЕС.".into(),
                solution: vec![
                    "Платформа на WordPress, e-commerce через WooCommerce.".into(),
                    "Онлайн-обучение на Tutor LMS.".into(),
                    "Мультиязычность через Polylang и доработка мультиязычности Tutor LMS.".into(),
                    "Кастомизация функционала под требования бизнеса.".into(),
                    "Развёртывание на сервере: Docker-окружение, почтовый сервер, reverse proxy.".into(),
                ],
                result: vec![
                    "Платформа продаёт курсы и услуги в одном месте.".into(),
                    "Мультиязычность работает корректно.".into(),
                    "Стабильная доступность сайта в разных странах.".into(),
                    "Архитектура готова к росту нагрузки.".into(),
                ],
                stack: vec![
                    "WordPress".into(),
                    "WooCommerce".into(),
                    "Tutor LMS".into(),
                    "Polylang".into(),
                    "Docker".into(),
                    "Reverse proxy".into(),
                ],
                image: "edu-platform-marketplace.webp".into(),
                site_url: Some("https://beauty-inna.pro".into()),
            },
            Project {
                slug: "wordpress-woocommerce-plugins".into(),
                title: "Плагин вместо ограничений WooCommerce".into(),
                niche: "WordPress · Плагины".into(),
                client: None,
                summary: "Стандартные возможности WooCommerce не покрывали логику клиента. Вместо доработок «поверх» и перегрузки корзины — отдельный плагин со своей архитектурой, аккуратно встроенный в работающий сайт.".into(),
                outcome: "Чистое архитектурное решение: стабильнее, проще в поддержке и развитии".into(),
                task: "Реализовать кастомную бизнес-логику, которая не укладывалась в стандартные возможности WooCommerce, не перегружая магазин и не ломая текущий функционал.".into(),
                solution: vec![
                    "Анализ ограничений WooCommerce под задачи бизнеса.".into(),
                    "Проектирование архитектуры кастомного решения.".into(),
                    "Разработка отдельного плагина WordPress.".into(),
                    "Реализация индивидуальной бизнес-логики.".into(),
                    "Интеграция с существующим сайтом без поломки текущего функционала.".into(),
                    "Задел на масштабируемость и дальнейшие доработки.".into(),
                ],
                result: vec![
                    "Логика вынесена в плагин, а не размазана по теме и ядру.".into(),
                    "Стабильность сайта выше.".into(),
                    "Поддержка проще, функционал развивается гибко.".into(),
                ],
                stack: vec![
                    "WordPress".into(),
                    "WooCommerce".into(),
                    "PHP".into(),
                ],
                image: "wordpress-woocommerce-plugins.webp".into(),
                site_url: None,
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
        assert!(c.services.len() >= 10, "каталог услуг подозрительно короткий");
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

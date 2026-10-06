# RULES: Архитектура проекта

## 📋 Содержание
- [Системная архитектура](#системная-архитектура)
- [Структура кода](#структура-кода)
- [Роутинг и маршруты](#роутинг-и-маршруты)
- [Слои приложения](#слои-приложения)
- [Компоненты Leptos UI](#компоненты-leptos-ui)
- [API Endpoints](#api-endpoints)
- [Потоки данных](#потоки-данных)

---

## Системная архитектура

### Общая схема
```
┌─────────────────────────────────────────────────┐
│                   ПОСЕТИТЕЛЬ                      │
│        (браузер / мобильное устройство)             │
└──────────────────────┬──────────────────────────┘
                       │ HTTPS
                       ▼
┌─────────────────────────────────────────────────┐
│              Docker Container                    │
│  ┌───────────────────────────────────────────┐  │
│  │     Rust/Leptos Server (port 8080)         │  │
│  │                                           │  │
│  │  ┌──────────┐  ┌──────────┐  ┌────────┐  │  │
│  │  │ Public   │  │ Admin    │  │ API    │  │  │
│  │  │ Router   │  │ Router   │  │ Layer  │  │  │
│  │  └──────────┘  └──────────┘  └────────┘  │  │
│  │     │              │           │          │  │
│  │     ▼              ▼           ▼          │  │
│  │  ┌───────────────────────────────────┐   │  │
│  │  │      Services Layer                │   │  │
│  │  │  ChatService | LLMService        │   │  │
│  │  │  MemoryService | NotificationSvc │   │  │
│  │  └──────────────┬────────────────────┘   │  │
│  └─────────────────┼────────────────────────┘  │
│                    │                            │
│         ┌──────────┼──────────┐                │
│         ▼          ▼          ▼                │
│    ┌────────┐ ┌────────┐ ┌────────┐            │
│    │SQLite  │ │OpenVik │ │  Redis │            │
│    │(chats) │ │ing     │ │ (cache)│            │
│    └────────┘ └────────┘ └────────┘            │
└─────────────────────────────────────────────────┘
                    │        │           │
              ┌─────▼──┐  ┌─▼──────┐  ┌─▼──────┐
              │Telegram│  │   VK   │  │  DeepSeek │
              │ Bot API│  │  API   │  │  OpenAI  │
              └────────┘  └────────┘  └─────────┘
```

### Принцип разделения ответственности
| Слой | Задачи | Не делает |
|---|---|---|
| **UI Layer** (Leptos) | Рендеринг страниц, формы, навигация | Логика бизнес-правил, доступ к БД |
| **API Layer** | HTTP обработчики, валидация, авторизация | Рендеринг UI, прямые SQL запросы |
| **Services Layer** | Бизнес-логика, orchestration | HTTP ответы, хранение данных |
| **Memory Layer** | OpenViking client, semantic search | Бизнес-логика, HTTP вызовы |
| **Storage Layer** | SQLite queries, Redis cache | Бизнес-правила |

---

## Структура кода

### src/main.rs — Точка входа
```rust
use std::env;

#[tokio::main]
async fn main() {
    // 1. Загрузка конфигурации (обязательно)
    let config = load_config().await;
    
    // 2. Инициализация сервисов
    let db = init_db(&config).await;          // SQLite
    let viking = init_openviking(&config).await; // OpenViking client
    let redis = init_redis(&config).await;     // Redis cache (опционально)
    
    // 3. Запуск сервера
    let app_state = AppState { db, viking, redis, config };
    start_server(app_state).await;
}

struct AppState {
    db: SqlitePool,
    viking: OpenVikingClient,
    redis: RedisClient,
    config: AppConfig,
}
```

### src/router.rs — Роутинг
```rust
use leptos::prelude::*;
use leptos_router::{Routes, Route, Router, HashLocationProvider};

fn main() {
    provide_meta_context();
    
    mount_to_body(|| {
        html! {
            <HashLocationProvider>
                <Router>
                    // Публичные маршруты
                    <Route path="/" view=Home />
                    <Route path="/chat" view=ChatBot />
                    <Route path="/services" view=Services />
                    <Route path="/projects" view=Projects />
                    <Route path="/status" view=Status />  // JSON response
                    <Route path="/contact" view=Contact />
                    
                    // Админ-маршруты (защита через middleware)
                    <Route path="/admin" view=AdminLogin />
                    <Route path="/admin/dashboard" view=AdminDashboard>
                        <ProtectedRoutes>
                            <Route path="*" view=AdminRoutes />
                        </ProtectedRoutes>
                    </Route>
                </Router>
            </HashLocationProvider>
        }
    });
}
```

### Структура модулей src/
```
src/
├── main.rs                 // Entry point, boot сервера
├── router.rs               // Leptos-роутинг
├── config.rs               // Загрузка и валидация config.toml / env vars
├── error.rs                // Единая система ошибок (AppError)
├── security.rs             // Заголовки безопасности ответов (CSP с nonce и др.)
│
├── ui/                     // ЛеPTos UI компоненты (SSR)
│   ├── shell.rs            // Обёртка документа: <head>, счётчики, мета-теги
│   ├── layout.rs           // Layout: header, footer, nav, main wrapper
│   ├── home.rs             // Hero-секция: имя, статус, 2 кнопки CTA
│   ├── services.rs         // Карточки услуг с иконками
│   ├── projects.rs         // Портфолио: кейсы "До/После"
│   ├── chat_widget.rs      // AI-чат виджет (embedded в страницы)
│   ├── status_badge.rs     // Статус занятости (inline компонент)
│   ├── contact.rs          // Контактная форма
│   └── shared/             // Общие компоненты
│       ├── button.rs       // Primary / Secondary кнопки
│       ├── card.rs         // Карточки с border-radius
│       ├── badge.rs        // Бейджи: статус, тег, метка
│       ├── navbar.rs       // Навигация (mobile-friendly)
│       └── footer.rs       // Footer: контакты, соцсети
│
├── admin/                  // Админ-панель
│   ├── login.rs            // Авторизация через токен
│   ├── dashboard.rs        // Аналитика: лиды, конверсии, графики
│   ├── chat_history.rs     // История чатов с фильтрами
│   └── settings.rs         // Управление контентом (через OpenViking)
│
├── api/                    // HTTP API endpoints (Hyper)
│   ├── mod.rs              // Регистрация routes для Hyper
│   ├── chat.rs             // POST /api/chat — запрос к чатботу
│   ├── lead.rs             // POST /api/lead — создание заявки
│   ├── status.rs           // GET  /api/status — статус занятости
│   ├── telegram.rs         // Webhook endpoint для Telegram
│   └── admin_auth.rs       // Middleware авторизации админа
│
├── memory/                 // OpenViking интеграция
│   ├── client.rs           // HTTP клиент к OpenViking server
│   ├── search.rs           // Wrapper для find/search/grep
│   ├── profile.rs          // Абстракция профиля автора
│   └── project.rs          // Абстракция проектов
│
├── llm/                    // LLM интеграция
│   ├── mod.rs              // Trait ChatProvider
│   ├── deepseek.rs         // DeepSeek V4 через OpenAI-compatible API
│   ├── provider.rs         // Общий интерфейс для провайдеров
│   └── prompt.rs           // Системные промпты, template builder
│
├── notification/           // Уведомления (Telegram + VK)
│   ├── mod.rs              // Unified notification trait
│   ├── telegram.rs         // Telegram Bot API
│   └── vk.rs               // VK Messages API
│
└── utils/                  // Утилиты
    ├── validator.rs        // Валидация email, форматов
    ├── logger.rs           // Structured logging (tracing)
    └── crypto.rs           // Шифрование для хранения данных
```

---

## Роутинг и маршруты

### Публичные маршруты
| Route | Метод | Описание | Ответ |
|-------|-------|----------|-------|
| `/` | GET | Главная: hero + статус + CTA кнопки | HTML |
| `/chat` | GET | Страница AI-чата | HTML |
| `/services` | GET | Список услуг с ценами (ориентировочными) | HTML |
| `/projects` | GET | Портфолио с кейсами "До/После" | HTML |
| `/blog` | GET | Список опубликованных статей | HTML |
| `/blog/:slug` | GET | Статья блога. Черновик/архив отдают 404 (не «мягкую» 200) | HTML |
| `/status.json` | GET | JSON endpoint статуса занятости | `application/json` |
| `/contact` | GET | Контактная форма + ссылки на мессенджеры | HTML |
| `/rss.xml`, `/feed.xml` | GET | RSS-лента блога с полным текстом в `content:encoded` | `application/rss+xml` |
| `/sitemap.xml` | GET | Карта сайта: статические страницы + опубликованные статьи | `application/xml` |
| `/robots.txt` | GET | Собирается на лету из `public_url`: адрес карты обязан совпадать с `/sitemap.xml`. См. `src/services/seo.rs` | `text/plain` |
| `/{имя из [seo].root_files}` | GET | Подтверждение прав (Яндекс/Google) и ключ IndexNow `{ключ}.txt`. Имена проверяет `SeoConfig::validate` | `text/html` / `text/plain` |
| `/media/:name` | GET | Изображения статей. `name` — `<24 hex от sha256>.<расширение>`; чужие имена отдают 404, ответ кэшируется на год и несёт `ETag` | `image/*` |
| `/og.png` | GET | Обложка для соцсетей (`og:image`): растровый PNG из `assets/og-default.png`, год кэша по версии в адресе | `image/png` |
| `/assets/style.css`, `/assets/main.js` | GET | Встроенные в бинарник CSS/JS. Адрес несёт `?v=<хеш содержимого>`; год кэша — только версионированному адресу | `text/css`, `application/javascript` |
| `/api/chat` | POST | API для AI-чата (JSON request/response) | `application/json` |
| `/api/lead` | POST | Создание заявки/лида | `application/json` |
| `/api/articles` | — | Публичного JSON-API статей нет: машинный доступ — RSS и outbox | — |
| `/api/settings-bot/webhook` | POST | Апдейты Telegram для бота настроек (смена статуса занятости). Нужен заголовок `X-Telegram-Bot-Api-Secret-Token`; иначе 404. См. `docs/settings-bot.md` | `200` |
| `/api/integrations/outbox` | GET | Фид событий кросспостинга для n8n. Заголовок `X-Api-Key`; без ключа в конфиге — 404. См. `docs/articles.md` | `application/json` |
| `/api/integrations/outbox/ack` | POST | Подтверждение доставки событий (идемпотентно) | `application/json` |
| `/api/integrations/outbox/fail` | POST | Сообщить о сбое доставки: инкремент попыток, `failed` после 5 | `application/json` |

### SEO-поверхность

Всё, что читают поисковики до контента, живёт в одном модуле — `src/services/seo.rs`.

| Что | Откуда берётся | Почему так |
|-----|----------------|------------|
| `/robots.txt` | `seo::robots_txt(public_url)` | Адрес карты сайта обязан совпадать с `PUBLIC_URL` и с тем, что отдаёт `/sitemap.xml`. Раньше это была константа в `assets/robots.txt`, и хост разошёлся (`inteli.dev.ru` вместо `inteli-dev.ru`) — ошибка тихая: поисковик просто не находит карту |
| `/{имя}` | `[seo].root_files` + файл ключа IndexNow | Подтверждение прав Яндекса и Google — файл с проверочной строкой. DNS TXT дешевле (не нужен рестарт), файл нужен, когда DNS недоступен |
| IndexNow | `[seo].indexnow_key` (env `INDEXNOW_KEY`) | Публикация, изменение и снятие статьи уведомляют Яндекс и Bing. Свежесть прямо влияет на попадание в генеративные ответы — см. `docs/seo-toolkit.md` |
| Счётчик Яндекс.Метрики | `[seo].yandex_metrika_id` (env `YANDEX_METRIKA_ID`) | Вставляется в `<head>` в `ui::shell::shell()`, поэтому номер публикуется в глобальную ячейку `seo::set_counters` при старте. Пусто — счётчика в разметке нет |
| Google tag (gtag.js) | `[seo].google_tag_id` (env `GOOGLE_TAG_ID`) | То же место и та же ячейка (`seo::Counters`). Счётчики живут вместе, чтобы не разъехаться: «Метрика считает, Google нет» заметно только по расхождению цифр в двух панелях |
| Заголовок, описание, canonical, OpenGraph | `seo::page_meta(путь, PUBLIC_URL, статья)` — вызывается из `ui::shell::shell()` | Раньше `title` и `description` были константами: краулер нашёл 100% дубликатов на семи страницах и ни одного `rel=canonical`/og-тега. Адрес берётся из пути запроса (`RequestUrl` в контексте Leptos), данные об авторе — из `SiteContent` (P3: факты о себе не хардкодятся), статья — из снимка `PublishedArticles` |
| Обложка для соцсетей | `assets/og-default.png` (`/og.png`), исходник — `assets/og-default.svg` | `og:image` обязан быть растровым: SVG не понимают ни Telegram, ни VK. Пересборка — `scripts/make-og-image.sh`. У статьи вместо общей картинки — её обложка, приведённая к абсолютному адресу |
| Заголовки безопасности | `src/security.rs` (CSP с nonce, X-Content-Type-Options, Referrer-Policy, Permissions-Policy, COOP, CORP) | CSP обязан содержать nonce конкретного ответа, а nonce генерирует Leptos при рендеринге, — значит, ставить его может только приложение. HSTS остаётся в nginx: это про транспорт |
| Кто может встраивать наши страницы | `frame-ancestors` в CSP (`'self'` + адреса Метрики) | **`X-Frame-Options` у нас нет намеренно**: он умеет только `DENY`/`SAMEORIGIN`, а список источников выражается лишь через `frame-ancestors`. Метрика открывает страницу в своём конструкторе целей и Вебвизоре — с одним `'self'` она показывала «Ошибка при загрузке страницы» |

Правила, которые нельзя нарушать:

- имя файла из `[seo].root_files` не может совпадать с `ROOT_ASSET_PATHS`
  (`src/api/mod.rs`) и обязано содержать расширение: страницы сайта — это
  сегменты без точки, поэтому файл с точкой с ними не столкнётся. В обоих
  случаях matchit паникует на конфликте маршрутов, и приложение не поднимается.
  Ловится в `SeoConfig::validate` и тестом `root_asset_paths_are_all_reserved`;
- уведомление IndexNow не влияет на публикацию: запрос уходит в фоне
  (`IndexNow::spawn_notify`), ошибка остаётся в логах, операция не падает;
- идентификаторы счётчиков вставляются в инлайновые `<script>`, поэтому алфавит
  у них узкий: номер Метрики — только цифры (`is_valid_metrika_id`), Google-тег —
  заглавные латиница, цифры и дефис (`is_valid_google_tag_id`). Невалидный
  идентификатор — это инъекция в страницу, поэтому `SeoConfig::validate` не даёт
  приложению стартовать с ним, а `Metrika::new`/`GoogleTag::new` в рантайме
  молча выключают свой счётчик, не трогая соседний;
- `<head>` собирается **только** в `ui::shell::shell()`. Leptos рендерит обёртку
  документа до страницы, поэтому страница не может дописать туда заголовок или
  canonical: всё, что зависит от адреса, вычисляется в обёртке из `RequestUrl`;
- заголовок и описание обязаны отличаться от страницы к странице (тест
  `static_pages_have_unique_titles_and_descriptions`). Одинаковый `title` — это
  не только замечание краулера, но и потерянные запросы;
- каждый инлайновый `<script>` несёт `nonce` из CSP того же ответа. Скрипт без
  nonce браузер не выполнит, а заметно это только по пустым графикам счётчиков;
- один заголовок — одно место. `X-Content-Type-Options`, CSP и остальные
  заголовки безопасности ставит приложение; в nginx их дублировать нельзя
  (`add_header` добавляет второй заголовок, и краулер видел `nosniff, nosniff`).
  Наоборот, HSTS ставит только nginx;
- кэш встроенного ресурса и версия в его адресе — одно решение
  (`api::assets::versioned`): год кэша разрешён только адресу с текущим `?v=`,
  иначе правка CSS не доедет до браузера;
- политика по ИИ-краулерам (кого пускать, кого нет) — отдельное решение
  владельца, а не побочный эффект правки robots.txt.

### Изображения статей

Всё, что связано с картинками, живёт в `src/services/media.rs` (проверка и
хранение) и `src/storage/media.rs` (таблица `media`). Подробности —
`docs/articles.md` §7.

| Что | Как | Почему так |
|-----|-----|------------|
| Формат | по «магическим» байтам, не по `Content-Type` запроса | файл с расширением `.png` и содержимым HTML, отданный с нашего домена, — это XSS на своём origin |
| Хранение | BLOB в той же SQLite, что и статьи | один volume, один бэкап, нет правил по путям и обхода каталога |
| Имя в URL | `<24 hex от sha256>.<расширение>` | адрес меняется вместе с содержимым, поэтому кэш можно держать год |
| Дедупликация | UNIQUE по `sha256` | та же картинка, загруженная дважды, не меняет адрес в опубликованной статье |
| Тело запроса | файл целиком, без multipart | загружается один файл без метаданных — multipart был бы лишним парсером |

Правила, которые нельзя нарушать:

- отказ на загрузке не должен ничего писать в текст статьи: ссылка на
  незагруженный файл выглядит как готовый текст, но ломается на сайте;
- `GET /media/:name` не трогает файловую систему — имя ищется в SQL по
  параметру, поэтому обход каталога невозможен. Формат имени всё равно
  проверяется (`is_valid_media_name`), чтобы мусор не доходил до базы;
- SVG не принимается осознанно: это изображение, которое умеет нести скрипт, а
  отдаётся оно с того же домена, что и сайт;
- загрузка картинки не уведомляет поисковики: IndexNow дёргается на публикацию
  статьи, а не на каждое вложение.

### Админ маршруты (требуют `Authorization: Bearer <ADMIN_TOKEN>`)
| Route | Метод | Описание |
|-------|-------|----------|
| `/admin` | GET | Вход по токену + дашборд (заявки, чаты, статьи, outbox) |
| `/api/admin/stats` | GET | Сводка: заявки/чаты за сегодня, время ответа, источники |
| `/api/admin/leads` | GET | Список заявок |
| `/api/admin/leads/:id` | PATCH | Смена статуса заявки |
| `/api/admin/chats` | GET | История чатов |
| `/api/admin/articles` | GET / POST | Список статей (счётчики по статусам) и создание |
| `/api/admin/articles/:id` | GET / PUT / PATCH / DELETE | Чтение, полное обновление, смена статуса, удаление |
| `/api/admin/article-preview` | POST | Рендер markdown в HTML тем же кодом, что и страница статьи |
| `/api/admin/media` | POST | Загрузка изображения статьи: тело — сам файл. Лимит тела поднят до 8 МБ только на этом маршруте |
| `/api/admin/outbox` | GET | Журнал событий кросспостинга (видно, что заберёт n8n) |

> ⚠️ **Синтаксис параметров пути:** axum 0.7 понимает только `:id`. Запись
> `{id}` появилась в axum 0.8 и здесь матчится как литеральный сегмент — такой
> маршрут молча отвечает 404. Именно так был сломан `PATCH /api/admin/leads/:id`.

### Middleware цепочка
```
Request → AuthCheck → RateLimit → LogRequest → Handler → Response
                                    │
                                    └─> AccessDenied (401/403)
```

---

## Слои приложения

### 1. HTTP Layer (Hyper)
Обработка входящих HTTP запросов:
```rust
// API handler для чата
async fn handle_chat(req: Request<Body>) -> Result<Response<Body>, AppError> {
    // 1. Валидация входных данных
    let ChatRequest { message, question_type } = parse_json::<ChatRequest>(req)?;
    
    // 2. Rate limiting check
    check_rate_limit(&client_ip)?)?;
    
    // 3. Передача в сервис слой
    let response = chat_service.answer(message, question_type).await?;
    
    // 4. Return JSON response
    Ok(json_response(200, &response))
}
```

### 2. Services Layer
Бизнес-логика:
```rust
// ChatService — orchestration чатбота
pub struct ChatService {
    viking_client: OpenVikingClient,
    llm_provider: Box<dyn LLMProvider>,
    redis_cache: RedisClient,
}

impl ChatService {
    pub async fn answer(&self, query: &str) -> Result<ChatResponse, AppError> {
        // 1. Try cache first (Redis)
        if let Some(cached) = self.redis_cache.get(&cache_key).await? {
            return Ok(cached);
        }
        
        // 2. Semantic search in OpenViking
        let context = self.viking_client.semantic_search(query, 5).await?;
        
        // 3. Build prompt with context + LLM call
        let prompt = build_prompt(query, &context)?;
        let llm_response = self.llm_provider.chat(prompt).await?;
        
        // 4. Cache result
        self.redis_cache.set(&cache_key, &llm_response, TTL_1H).await?;
        
        // 5. Save to SQLite
        self.save_to_db(query, &llm_response).await?;
        
        Ok(llm_response)
    }
}

// NotificationService — отправка лидов
pub async fn notify_on_lead(lead: &Lead) -> Result<(), AppError> {
    let (tg_result, vk_result) = tokio::join!(
        notify_telegram(lead),
        notify_vk(lead),
    );
    
    // Log both results, but don't fail if one is down
    match (tg_result, vk_result) {
        (Ok(_), Ok(_)) => info!("Leads sent to Telegram + VK"),
        (Ok(_), Err(e)) => warn!("Telegram OK, VK failed: {}", e),
        (Err(e), Ok(_)) => warn!("VK OK, Telegram failed: {}", e),
        (Err(tg_err), Err(vk_err)) => error!("Both notifications failed: TG={}, VK={}", tg_err, vk_err),
    }
    
    Ok(())
}
```

### 3. Memory Layer (OpenViking)
```rust
// OpenVikingClient — HTTP wrapper для MCP endpoints
pub struct OpenVikingClient {
    base_url: String,
    client: reqwest::Client,
}

impl OpenVikingClient {
    // Semantic search across all resources
    pub async fn semantic_search(&self, query: &str, limit: usize) 
        -> Result<Vec<MemoryResult>, AppError> 
    {
        let url = format!("{}/api/find", self.base_url);
        let params = serde_json::json!({
            "query": query,
            "limit": limit,
            "min_score": 0.35,
            "context_type": ["memory", "resource"],
        });
        
        let response = self.client.post(url)
            .json(&params)
            .send()
            .await?;
            
        Ok(response.json().await?)
    }
    
    // Read specific URI
    pub async fn read_uri(&self, uri: &str) -> Result<String, AppError> {
        let url = format!("{}/api/read", self.base_url);
        let response = self.client.post(url)
            .json(&serde_json::json!({ "uris": [uri] }))
            .send()
            .await?;
        
        Ok(response.text().await?)
    }
    
    // Write/Update URI (via admin only)
    pub async fn write_uri(&self, uri: &str, content: &str) 
        -> Result<(), AppError> 
    {
        let url = format!("{}/api/write", self.base_url);
        self.client.post(url)
            .json(&serde_json::json!({
                "uri": uri,
                "content": content,
            }))
            .send()
            .await?;
        
        Ok(())
    }
}

// Profile abstraction — извлекает данные профиля из OpenViking
pub struct AuthorProfile {
    pub name: String,
    pub title: String,
    pub experience_years: u8,
    pub skills: Vec<String>,
    pub contact_email: String,
    pub contact_telegram: String,
    pub contact_vk: String,
}

impl AuthorProfile {
    pub async fn load_from_openviking(viking: &OpenVikingClient) 
        -> Result<Self, AppError> 
    {
        let content = viking.read_uri("viking://user/inteli-dev/profile/main.md").await?;
        
        // Parse markdown content into struct (simple key-value format)
        Self::parse_from_markdown(&content)
    }
}
```

### 4. LLM Layer
```rust
// Trait для абстракции провайдеров LLM
pub trait ChatProvider: Send + Sync {
    async fn chat(&self, prompt: String) -> Result<String, AppError>;
    async fn chat_with_system(&self, system: String, user_msg: String) 
        -> Result<String, AppError>;
}

// DeepSeek V4 через OpenAI-compatible API
pub struct DeepSeekProvider {
    api_key: String,
    base_url: String,  // Например: https://api.deepseek.com/v1
    model: String,     // "deepseek-chat" или "deepseek-v4"
}

impl ChatProvider for DeepSeekProvider {
    async fn chat_with_system(&self, system: String, user_msg: String) 
        -> Result<String, AppError> 
    {
        let payload = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user_msg},
            ],
            "temperature": 0.7,
            "max_tokens": 1000,
        });
        
        let response = reqwest::Client::new()
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload)
            .send()
            .await?;
            
        let body = response.json::<serde_json::Value>().await?;
        
        body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or(AppError::EmptyResponse)?
            .to_string()
    }
}
```

### 5. Storage Layer (SQLite)
```rust
// SQLX queries для хранения чатов и лидов
pub struct ChatEntry {
    pub id: i64,
    pub message: String,
    pub response: String,
    pub question_type: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub client_ip: Option<String>,  // anonymized (first 2 octets)
}

impl ChatEntry {
    pub async fn save(&self, pool: &SqlitePool) -> Result<(), AppError> {
        sqlx::query!(
            r#"INSERT INTO chats (message, response, question_type, created_at)
               VALUES ($1, $2, $3, $4)"#,
            self.message,
            self.response,
            self.question_type,
            self.created_at,
        ).execute(pool).await?;
        
        Ok(())
    }
    
    pub async fn find_recent(pool: &SqlitePool, limit: usize) 
        -> Result<Vec<Self>, AppError> 
    {
        let rows = sqlx::query_as!(
            ChatEntry,
            r#"SELECT id, message, response, question_type, created_at, client_ip
               FROM chats ORDER BY created_at DESC LIMIT $1"#,
            limit as i64,
        ).fetch_all(pool).await?;
        
        Ok(rows)
    }
}

// Lead entry (заявка/лид)
pub struct Lead {
    pub id: i64,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub message: String,
    pub source: String,  // "form", "chat", "telegram"
    pub status: String,  // "new", "processed", "converted"
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Lead {
    pub async fn save(&self, pool: &SqlitePool) -> Result<i64, AppError> {
        let result = sqlx::query!(
            r#"INSERT INTO leads (name, email, phone, message, source, status, created_at)
               VALUES ($1, $2, $3, $4, $5, 'new', $6)"#,
            self.name,
            self.email,
            self.phone,
            self.message,
            self.source,
            self.created_at,
        ).execute(pool).await?;
        
        Ok(result.last_insert_rowid())
    }
}

// SQL schema
pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS chats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    message TEXT NOT NULL,
    response TEXT NOT NULL,
    question_type TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    client_ip TEXT  -- anonymized
);

CREATE TABLE IF NOT EXISTS leads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT,
    phone TEXT,
    message TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'form',
    status TEXT NOT NULL DEFAULT 'new',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_chats_created ON chats(created_at);
CREATE INDEX IF NOT EXISTS idx_leads_status ON leads(status);
CREATE INDEX IF NOT EXISTS idx_leads_source ON leads(source);
"#;
```

---

## Компоненты Leptos UI

### Home component (главная страница)
```rust
// ui/home.rs — hero секция с двумя CTA кнопками
#[component]
fn Home() -> impl IntoView {
    let profile = load_profile();  // из OpenViking или кэша
    let status = get_availability_status().await;
    
    view! {
        <section class="hero">
            <!-- Header с навигацией -->
            <nav>...</nav>
            
            <!-- Hero content -->
            <div class="hero-content">
                <h1>{profile.name}</h1>
                <p class="subtitle">{profile.title}</p>
                
                <!-- Status badge -->
                <StatusBadge status={status} />
                
                <!-- DUAL CTA BUTTONS -->
                <div class="cta-buttons">
                    <PrimaryButton 
                        text="💬 Задать вопрос" 
                        href="/chat"
                    />
                    <SecondaryButton 
                        text="📩 Оставить заявку" 
                        href="#contact-form"
                    />
                </div>
            </div>
            
            <!-- Services preview (3 карточки) -->
            <ServicesPreview />
        </section>
    }
}

// ui/shared/button.rs — универсальные кнопки
#[component]
fn PrimaryButton(text: String, href: String) -> impl IntoView {
    view! {
        <a href={href} class="btn btn-primary">
            {text}
        </a>
    }
}

#[component]
fn SecondaryButton(text: String, href: String) -> impl IntoView {
    view! {
        <a href={href} class="btn btn-secondary">
            {text}
        </a>
    }
}
```

---

## API Endpoints

### POST /api/chat — AI Chat endpoint
**Request:**
```json
{
    "message": "Сколько стоит SEO-продвижение?",
    "session_id": null,  // optional, for tracking conversation
    "question_type": "preset"  // or "free", "analysis", "lead_request"
}
```

**Response:**
```json
{
    "answer": "SEO-продвижение начинается от 50 000 ₽ в месяц. Стоимость зависит от...",
    "suggested_next": [
        { "text": "Рассчитать стоимость для моего сайта", "type": "lead_request" },
        { "text": "Посмотреть примеры работ", "type": "link: /projects" }
    ],
    "citations": [
        "viking://user/inteli-dev/services/seo-audit.md"
    ]
}
```

### POST /api/lead — Lead submission
**Request:**
```json
{
    "name": "Иван Иванов",
    "email": "ivan@example.com",
    "phone": "+7 (999) 123-45-67",
    "message": "Нужно продвинуть сайт интернет-магазина электроники",
    "source": "form"  // or "chat", "telegram"
}
```

**Response:**
```json
{
    "success": true,
    "message": "Ваша заявка принята! Я свяжусь с вами в ближайшее время."
}
```

### GET /api/status — Availability status
**Response:**
```json
{
    "status": "available",  // "available" | "busy" | "full"
    "label": "Приём заявок: открыт",  // подлежащее + состояние
    "icon_name": "circle-check",
    "availability_date": "2026-10-15",
    "current_projects": 3,
    "next_free_slot": "начало ноября 2026",
    "updated_at": "2026-09-07T10:00:00Z"
}
```

`label` собирается из словаря `services/status.rs::presentation` — того же, что
кормит бейдж в шапке, ответ чата «Когда свободны?» и Telegram-бота. Неизвестный
`status` трактуется как «закрыт», чтобы опечатка в БД не обещала свободные слоты.

### GET /api/status/embed.js — Embed widget для внешних сайтов
```javascript
// Возвращает JS snippet:
document.write('<span class="inteli-status-available">🟢 Свободен для проектов</span>');
```

---

## Потоки данных

### Поток 1: Посетитель → Чат → Лид
```
Посетитель нажимает "Задать вопрос" 
    → UI отправляет POST /api/chat { message }
        → ChatService получает запрос
            ├── Redis: check cache (TTL 1h) → hit? return cached
            └── miss → OpenViking semantic_search(query)
                → LLM provider.chat(prompt_with_context)
                → Save to SQLite
                → Add CTA "Оставить заявку?" в ответ
        → UI показывает ответ + кнопку заявки
```

### Поток 2: Посетитель → Заявка → Уведомление
```
Посетитель заполняет форму на /contact
    → POST /api/lead { name, email, phone, message }
        → LeadService.save() 
            ├── SQLite: INSERT INTO leads
            └── NotificationService.notify_all(lead)
                ├── Telegram Bot API: sendMessage to admin chat
                └── VK Messages API: send to admin user_id
        → Return "thank you" message to user
```

### Поток 3: Админ → Обновление профиля → OpenViking
```
Админ вводит данные в /admin/settings
    → PUT /api/admin/profile { name, skills, projects }
        ├── Validate input
        ├── Convert to markdown format
        └── viking_client.write_uri("viking://user/inteli-dev/profile/main.md", content)
            → Invalidate Redis cache for profile data
```

### Поток 4: Telegram Webhook → Обработка входящих сообщений
```
Telegram Bot receives message (webhook POST)
    → api/telegram handler
        ├── Parse message (text, inline_keyboard response)
        ├── If "lead confirmation" → update Lead status in SQLite
        └── Send acknowledgment back to Telegram
```

---

*См. также: [docker-build.md](./docker-build.md), [memory-storage.md](./memory-storage.md)*

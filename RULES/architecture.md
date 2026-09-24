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
│
├── ui/                     // ЛеPTos UI компоненты (SSR)
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
| `/status.json` | GET | JSON endpoint статуса занятости | `application/json` |
| `/contact` | GET | Контактная форма + ссылки на мессенджеры | HTML |
| `/api/chat` | POST | API для AI-чата (JSON request/response) | `application/json` |
| `/api/lead` | POST | Создание заявки/лида | `application/json` |
| `/api/settings-bot/webhook` | POST | Апдейты Telegram для бота настроек (смена статуса занятости). Нужен заголовок `X-Telegram-Bot-Api-Secret-Token`; иначе 404. См. `docs/settings-bot.md` | `200` |

### Админ маршруты (требуют авторизации)
| Route | Метод | Описание |
|-------|-------|----------|
| `/admin` | GET | Логин страница |
| `/admin/dashboard` | GET | Аналитика и метрики |
| `/admin/chats` | GET/POST | История чатов, фильтры |
| `/admin/chats/:id` | GET | Детали конкретного чата |
| `/admin/leads` | GET | Список всех лидов |
| `/admin/settings` | POST | Обновление профиля через OpenViking |

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
    "availability_date": "2026-10-15",
    "current_projects": 3,
    "next_free_slot": "начало ноября 2026",
    "updated_at": "2026-09-07T10:00:00Z"
}
```

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

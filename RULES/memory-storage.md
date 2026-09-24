# RULES: Хранение данных для ИИ

## 📋 Содержание
- [Основной принцип хранения](#основной-принцип-хранения)
- [Хранение в OpenViking (`viking://` URIs)](#хранение-в-openviking-viking-uri-)
- [Альтернативные и дополнительные решения](#альтернативные-и-дополнительные-решения)
- [Матрица хранения данных](#матрица-хранения-данных)
- [Форматы файлов OpenViking](#форматы-файлов-openviking)
- [Стратегии кеширования](#стратегии-кеширования)

---

## Основной принцип хранения

> **"Один источник правды — одна система для одного типа данных."**

Каждый тип данных хранится в единственном месте:
- Знания об авторе → **OpenViking** (семантический поиск)
- История чатов → **SQLite** (структурированные записи)
- Заявки/лиды → **SQLite** + уведомление через API
- Кэш ответов LLM → **Redis** (скорость под-мс)
- Статические файлы → **Файловая система**

---

## Хранение в OpenViking (`viking://` URIs)

### Структура URI

```
viking://user/inteli-dev/
├── profile/
│   ├── main.md                 — главный профиль автора
│   ├── skills.md               — навыки и компетенции
│   └── experience.md           — опыт, карьера, достижения
├── services/
│   ├── overview.md             — обзор услуг
│   ├── seo-audit.md            — SEO-аудит: что входит, цены
│   ├── promotion.md            — Продвижение в поисковиках
│   ├── technical-seo.md        — Техническая оптимизация
│   ├── content-strategy.md     — Контент-стратегия
│   └── analytics.md            — Аналитика и отчёты
├── projects/
│   ├── index.md                — оглавление проектов
│   ├── case-001.md             — кейс: интернет-магазин
│   ├── case-002.md             — кейс: корпоративный сайт
│   └── ...
├── policies/
│   ├── availability.md         — текущая занятость и расписание
│   ├── pricing-notes.md        — заметки о ценообразовании (для AI)
│   └── communication.md        — как общаемся с клиентами
├── prompts/
│   ├── chatbot-system.md       — основной системный промпт
│   ├── analysis-prompt.md      — промпт для анализа проектов
│   └── lead-qualification.md   — промпт для квалификации лида
└── templates/
    ├── lead-reply.md           — шаблон ответа на заявку
    ├── project-analysis.md     — шаблон отчёта по анализу проекта
    └── follow-up.md            — шаблон follow-up сообщения
```

### Пример: профиль main.md

```markdown
---
uri: viking://user/inteli-dev/profile/main.md
tags: [profile, author, bio]
last_updated: 2026-09-07T10:00:00Z
---

# Профиль автора

## Основная информация
- **Имя**: [Ваше Имя]
- **Должность**: Специалист по продвижению сайтов (SEO)
- **Опыт**: 10 лет в поисковой оптимизации
- **Локация**: Россия, удалённая работа
- **Языки**: Русский, English (B2)

## Ключевые компетенции
SEO-аудит · Продвижение Яндекс/Google · Техническое SEO
Контент-стратегия · Аналитика · Связывание с внешними сайтами

## Целевая аудитория
Владельцы бизнеса, маркетологи, разработчики

## Контактные данные
- Telegram: @username
- Email: email@example.com  
- VK: vk.com/username
- Телефон: +7 (XXX) XXX-XX-XX

## Текущий статус
Занятость: 5-7 проектов одновременно
Средний чек проекта: от 50 000 ₽
Время ответа на заявку: в течение 24 часов
```

### Пример: услуги seo-audit.md

```markdown
---
uri: viking://user/inteli-dev/services/seo-audit.md
tags: [service, audit, pricing]
last_updated: 2026-09-07T10:00:00Z
---

# SEO-аудит сайта

## Что входит
1. Технический аудит (скорость, мобильная версия, indexability)
2. Анализ контента и ключевых слов
3. Проверка ссылочного профиля
4. Аудит юзабилити и UX
5. Аналитика конкурентов

## Результаты аудита
- Полный отчёт с приоритетами (Critical / High / Medium / Low)
- Excel/CSV таблица всех发现的问题
- Презентация основных выводов (PDF)
- Персональная консультация по результатам

## Стоимость
- Базовый аудит: 15 000 ₽ (3-5 дней)
- Полный аудит: 35 000 ₽ (7-10 дней)  
- Глубокий аудит + план работ: 50 000 ₽ (14 дней)

## Для кого
Сайты с трафиком < 1000 визитов/день, у которых нет заявок из поиска.
```

### Пример: системный промпт chatbot-system.md

```markdown
---
uri: viking://user/inteli-dev/prompts/chatbot-system.md
tags: [prompt, system, chatbot]
last_updated: 2026-09-07T10:00:00Z
---

# Системный промпт AI-чата

Ты — AI-ассистент специалиста по продвижению сайтов с 10-летним опытом.
Твоя задача — помочь посетителю сайта быстро получить ответы на вопросы 
и, при необходимости, оставить заявку на консультацию.

## Твои знания (извлечены из открытых источников и контекста):
{context_from_openviking}

## Твои правила:
1. Отвечай кратко и по делу (максимум 3-4 предложения для простых вопросов)
2. Если не знаешь ответа — скажи честно и предложи связаться с автором
3. Всегда предлагай следующий шаг («Хотите рассчитать стоимость?» / «Оставить заявку?»)
4. Не обещай результатов, которые не могут быть гарантированы
5. Упоминайте конкретные цифры из контекста (опыт, кейсы, расценки)
6. Если посетитель спрашивает о ценах — давай ориентировочный диапазон и уточняй

## Тон коммуникации:
- Деловой, но дружелюбный
- Уверенный, но не высокомерный
- Готовый помочь
```

---

## Альтернативные и дополнительные решения

### 1. SQLite с WAL mode — для структурированных данных

**Когда использовать:** Чаты, лиды, метрики — всё что нужно быстро запросить по SQL.

| Параметр | Значение |
|---|---|
| Формат файла | `.sqlite` (один файл на БД) |
| Режим записи | WAL (Write-Ahead Logging) для concurrent reads |
| Размер за год | ~10-50 MB при активной нагрузке |
| Rust драйвер | `sqlx` с TLS |
| Backup | Копирование файла или `.backup` command |

```sql
-- SQLite schema для чатов и лидов
CREATE TABLE IF NOT EXISTS chats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_message TEXT NOT NULL,
    bot_response TEXT NOT NULL,
    question_type TEXT CHECK(question_type IN ('preset', 'free', 'analysis', 'lead_request')),
    session_id TEXT,
    client_hash TEXT,  -- anonymized IP hash (SHA256 first 32 chars)
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    response_time_ms INTEGER  -- для аналитики производительности
);

CREATE TABLE IF NOT EXISTS leads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT CHECK(length(email) <= 254),
    phone TEXT,
    message TEXT NOT NULL,
    source TEXT DEFAULT 'form' CHECK(source IN ('form', 'chat', 'telegram', 'vk')),
    status TEXT DEFAULT 'new' CHECK(status IN ('new', 'processing', 'contacted', 'converted', 'dismissed')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Включение WAL mode (настраивается при открытии connection pool)
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
PRAGMA cache_size=-64000;  -- 64MB cache
```

**Ещё две таблицы (статьи блога и кросспостинг):**

```sql
-- Контент, который пишет владелец из админки.
CREATE TABLE IF NOT EXISTS articles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    summary TEXT NOT NULL DEFAULT '',
    body_markdown TEXT NOT NULL,
    cover_image_url TEXT,
    tags TEXT NOT NULL DEFAULT '[]',   -- JSON-массив строк
    status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft', 'published', 'archived')),
    published_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'admin' CHECK(source IN ('admin', 'n8n', 'import')),
    external_id TEXT,                  -- ключ идемпотентности внешнего импорта
    canonical_url TEXT
);

-- Transactional outbox: события для кросспостинга через n8n.
-- Пишется В ОДНОЙ ТРАНЗАКЦИИ со статьёй, поэтому «опубликовано, но событие
-- потерялось» невозможно. article_id намеренно без FOREIGN KEY: лог доставки
-- обязан переживать удаление статьи.
CREATE TABLE IF NOT EXISTS article_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    article_id INTEGER NOT NULL,
    article_slug TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK(event_type IN
        ('article.published', 'article.updated', 'article.unpublished', 'article.deleted')),
    payload TEXT NOT NULL,             -- JSON-снимок статьи
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'delivered', 'failed')),
    attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TEXT,              -- задел под push с backoff
    created_at TEXT NOT NULL,
    delivered_at TEXT,
    last_error TEXT
);
```

Подробности: `docs/articles.md`. Слой доступа — `src/storage/article.rs`.

> **Статус занятости живёт в SQLite, а не в OpenViking** (`site_settings`,
> ключ → JSON). Это состояние сервиса, которое меняется несколько раз в день и
> должно переживать рестарт, а не семантическое знание об авторе. См.
> `src/settings.rs` и `docs/settings-bot.md`.

### 2. Redis — для кеширования

**Когда использовать:** Кэш ответов LLM, rate limiting, сессии админа.

| Параметр | Значение |
|---|---|
| Образ Docker | `redis:7-alpine` (~15MB) |
| TTL по умолчанию | 3600s (1 час) для ответов чата |
| Память при полной нагрузке | ~1-2 GB (зависит от нагрузки) |
| Persistence | AOF + RDB snapshots |

**Ключи Redis:**
```
# Кэш ответов LLM
k:chat:{md5(question)} → {json: answer, ttl: 1h}

# Rate limiting
rl:chat:{ip_hash} → {count: N, window_start: timestamp}

# Сессия админа  
sess:admin:{token_hash} → {user_id, created_at, expires_at}

# Статус занятости (кэш)
st:availability → {status, date, updated_at}

# Популярный вопрос статистика
sc:popular_question:{preset_index} → {count, last_used}
```

### 3. File-based JSON — для минимальных данных

**Когда использовать:** robots.txt, sitemap.xml, manifest.json (PWA).

```json
// public/manifest.json — PWA манифест
{
    "name": "inteli.dev — Специалист по SEO",
    "short_name": "SEO Expert",
    "description": "Специалист по продвижению сайтов с 10-летним опытом",
    "start_url": "/",
    "display": "standalone",
    "theme_color": "#1a1a2e",
    "background_color": "#ffffff",
    "icons": [
        { "src": "/icon-192.png", "sizes": "192x192", "type": "image/png" },
        { "src": "/icon-512.png", "sizes": "512x512", "type": "image/png" }
    ]
}
```

### 4. Альтернативы OpenViking (на будущее)

Если в какой-то момент OpenViking недоступен или нужно дублирование:

| Решение | Плюсы | Минусы | Когда использовать |
|---|---|---|---|
| **LanceDB** | Rust-native, embedded, vector search | Локально только, нет remote API | Small deployment без отдельного сервера памяти |
| **SQLite + vec-extensions** | Одна БД для всего, SQL + vectors | Менее зрелый ecosystem | Простая архитектура "всё в одном" |
| **ChromaDB** | Простой Python API, good embedding support | Python dependency, нет Rust native драйвера | Если уже есть Python-микросервисы |
| **Qdrant Cloud** | Managed, high performance, rust-native server | Внешний сервис, стоимость | Production с высоким трафиком |
| **pgvector (PostgreSQL)** | SQL + vectors в одном, enterprise ready | Тяжелее SQLite, нужен PostgreSQL сервер | Если проект масштабируется до PostgreSQL |

---

## Матрица хранения данных

| Тип данных | Основное хранилище | Формат | Доступ из кода | TTL / Lifespan |
|---|---|---|---|---|
| Профиль автора | OpenViking | Markdown → viking://uri | `memory/profile.rs` | Постоянно, обновляется вручную |
| Навыки и компетенции | OpenViking | Markdown → viking://uri | `memory/skills.rs` | Постоянно |
| Страница услуг | OpenViking | Markdown → viking://uri | `memory/services.rs` | Постоянно |
| Кейсы проектов | OpenViking + SQLite | MD в Viki, метрики в SQL | `memory/project.rs` | Постоянно |
| Статус занятости | OpenViking → Redis кэш | JSON / text | `api/status.rs` | Кэш 1 час, источник обновляется при необходимости |
| Чаты пользователей | SQLite | Таблица `chats` | `storage/chat_repository.rs` | 90 дней (автоматическая очистка) |
| Заявки/лиды | SQLite | Таблица `leads` | `storage/lead_repository.rs` | Постоянно |
| Кэш ответов LLM | Redis | JSON ключ-значение | `cache/llm_cache.rs` | 1 час |
| Session admin | Redis | Token → user info | `api/admin_auth.rs` | 24 часа |
| Rate limiting counters | Redis | Integer counter | Middleware `rate_limit.rs` | sliding window 1 мин |
| Логирование | Файл (JSON lines) | JSON lines | Structured logger | 30 дней, затем архив |

---

## Форматы файлов OpenViking

### Правила форматирования

1. **Markdown** — основной формат для всех URI
2. **YAML frontmatter** — метаданные в начале файла:
   ```yaml
   ---
   uri: viking://user/inteli-dev/profile/main.md
   tags: [profile, author]
   last_updated: 2026-09-07T10:00:00Z
   ---
   ```
3. **Секции через заголовки** — H2 (#) для разделов, H3 (##) для подразделов
4. **Ключевые данные в bullet-lists** — для структурированной читаемости

### Правила написания для AI

Каждый файл, хранящийся в OpenViking, должен быть оптимизирован для:
1. **Семантического поиска** — использовать полные фразы, естественный язык
2. **Контекстной подстановки** — ответы должны быть автономными (не зависеть от других файлов)
3. **Актуальности** — обновлять дату `last_updated` при каждом изменении

### Пример оптимального файла для AI

```markdown
---
uri: viking://user/inteli-dev/services/seo-audit.md
tags: [service, audit, pricing, seo]
last_updated: 2026-09-07T10:00:00Z
---

# SEO-аудит сайта — описание услуги

## Что такое SEO-аудит
SEO-аудит — это комплексный анализ сайта на предмет факторов, влияющих 
на позиции в поисковых системах Яндекс и Google. Аудит помогает понять, 
почему сайт не получает достаточно трафика из органической выдачи, 
и определить конкретные шаги для исправления ситуации.

## Что входит в базовый аудит (15 000 ₽)
Базовый SEO-аудит включает анализ ключевых технических параметров:
- Скорость загрузки сайта (Core Web Vitals)
- Мобильная адаптация и responsiveness
- Настройка robots.txt, sitemap.xml, canonical URL
- Проверка индексации и ошибок 4xx/5xx
- Базовый анализ мета-тегов (title, description)

## Что входит в полный аудит (35 000 ₽)
Полный SEO-аудит включает всё из базового + глубокий анализ:
- Полный технический аудит с инструментами Screaming Frog / Ahrefs
- Анализ контентной стратегии и ключевых слов
- Проверка ссылочного профиля (количество, качество доноров)
- UX/UI аудит на основе heatmap-анализа
- Аудит конкурентов в нише клиента

## Что входит в глубокий аудит (50 000 ₽)
Глубокий аудит включает полный аудит + разработку плана работ:
- Полный технический и контентный аудит
- Разработка пошагового плана продвижения на 6 месяцев
- Расчёт ROI и прогнозируемого трафика по каждому изменению
- Консультация с командой разработки (объяснение, что делать)

## Когда нужен SEO-аудит
SEO-аудит рекомендуется если:
- Трафик из поиска стагнирует или падает более 3 месяцев
- Сайт не входит в ТОП-10 по основным запросам
- После изменения структуры сайта или миграции на новый домен
- Вы планируете запуск нового интернет-магазина или лендинга
```

---

## Стратегии кеширования

### Уровень 1: HTTP Cache Headers
```rust
// Все публичные страницы — кэшировать на 1 час (CDN / browser)
response.headers.insert(
    "Cache-Control", 
    "public, max-age=3600, s-maxage=3600"
);

// API статус — кэшировать на 5 минут (часто обновляется)
response.headers.insert(
    "Cache-Control", 
    "public, max-age=300, s-maxage=300"
);

// Статические ассеты — кэшировать навсегда с хешем в имени
response.headers.insert(
    "Cache-Control", 
    "public, max-age=31536000, immutable"
);
```

### Уровень 2: Redis Cache
```rust
// Chat answer cache
pub async fn get_or_compute_chat_response(
    query: &str, 
    redis: &RedisClient, 
    compute_fn: impl FnOnce() -> Fut,
) -> Result<String, AppError> {
    let cache_key = format!("chat:{}", sha256(query.as_bytes())[..16].to_string());
    
    // Try to get from cache first
    if let Some(cached) = redis.get(&cache_key).await? {
        return Ok(cached);  // Cache HIT — мгновенный ответ
    }
    
    // Cache MISS — compute fresh answer
    let answer = compute_fn().await?;
    
    // Store in cache for 1 hour
    redis.set_ex(&cache_key, &answer, 3600).await?;
    
    Ok(answer)
}
```

### Уровень 3: Application-level Cache (in-memory)
```rust
// Profile data — кэш на уровне приложения (обновляется при reload)
static PROFILE_CACHE: std::sync::OnceLock<AuthorProfile> = std::sync::OnceLock::new();

pub fn get_cached_profile() -> &'static AuthorProfile {
    PROFILE_CACHE.get_or_init(|| {
        // Load once at startup from OpenViking or fallback file
        tokio::task::block_in_place(|| {
            runtime().block_on(async {
                load_profile_from_openviking().await
                    .unwrap_or_else(|_| default_fallback_profile())
            })
        })
    })
}
```

### Стратегия инвалидации кэша
| Событие | Что инвалидируется | Как |
|---|---|---|
| Admin обновил профиль | Profile cache (L3) | Reload при следующем запросе + Redis DEL key |
| Admin изменил статус занятости | Status cache (L2, L1) | Redis `DEL st:availability` |
| Новый чат сохранён | Ничего не инвалидируется | Чаты не кэшируются |
| Лид создан | Ничего не инвалидируется | Лиды не кэшируются |

---

*См. также: [ai-fabric.md](./ai-fabric.md), [architecture.md](./architecture.md)*

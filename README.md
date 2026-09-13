# inteli.dev.ru

Личный сайт-визитка специалиста по продвижению сайтов (SEO). Цель — **машина
генерации лидов** с минимальным трением: пара кнопок → заявка → уведомление в
Telegram + VK.

Философия: **«Минимум кнопок и текста, максимум полезности и смысла»** (см. `AGENT.md`).

## Стек

| Компонент | Технология |
|---|---|
| UI | Leptos (SSR) + ванильный JS |
| HTTP | axum (hyper) + tokio |
| Хранилище | SQLite (WAL, sqlx) |
| Память ИИ | OpenViking (`viking://`) с fallback |
| LLM | DeepSeek V4 (OpenAI-compatible) |
| Уведомления | Telegram Bot API + VK Messages API |
| Деплой | Docker multi-stage (~35 MB runtime) |

## Структура

```
src/
├── main.rs            — boot: config → слои → сервер
├── lib.rs             — библиотека (модули, тестируются интеграционно)
├── config.rs          — конфигурация (config.toml + env)
├── error.rs           — AppError (thiserror)
├── state.rs           — AppState (axum state)
├── api/               — HTTP-хендлеры (chat, lead, status, health, admin)
├── services/          — бизнес-логика (чат, заявки, статус)
├── storage/           — SQLite (chats, leads)
├── memory/            — OpenViking client + fallback-контент
├── llm/               — ChatProvider trait + DeepSeek + промпты
├── notification/      — Telegram + VK
├── cache.rs           — in-memory кэш + rate limiter
├── utils/             — валидация, анонимизация IP
└── ui/                — Leptos SSR-страницы (включая /admin)
assets/                — style.css (дизайн-токены), main.js, статика
tests/                 — интеграционные тесты API
RULES/                 — правила проекта (архитектура, UI, …)
docs/ADR.md            — архитектурные решения
```

## Быстрый старт

### Локально (нужен Rust 1.85+)

```bash
cp config.toml.example config.toml   # при необходимости
cargo run
# → http://localhost:8080
```

Секреты задаются через env (`LLM_API_KEY`, `TELEGRAM_BOT_TOKEN`, `ADMIN_TOKEN`, …)
или `config.toml`. Без ключей сайт работает: чат отвечает fallback-ответами,
уведомления пропускаются.

### Docker (dev)

```bash
cp .env.example .env    # заполнить секреты
docker compose up --build
# → http://localhost:8080
```

### Docker (prod)

Продакшен — отдельный файл `docker-compose.prod.yml`: приложение за Nginx,
трафик и TLS — через внешний Traefik (сеть `traefik-public`), сертификат
Let's Encrypt выпускается Traefik автоматически.

```bash
cp .env.prod.example .env.prod    # заполнить секреты
docker compose --env-file .env.prod -f docker-compose.prod.yml up -d --build
```

Требования к серверу:

- внешняя docker-сеть Traefik: `docker network create traefik-public` (один раз);
- A-запись `inteli-dev.ru` (и `www`) на этот сервер — без неё ACME HTTP-01 не пройдёт.

Конфигурация приложения — `config/config.prod.toml` (монтируется read-only как
`/app/config.toml`, секретов в нём нет). Подробности — `RULES/docker-build.md`.

## API

| Метод | Путь | Описание |
|---|---|---|
| GET | `/api/health` | health-check |
| GET | `/api/status` | статус занятости |
| POST | `/api/chat` | AI-чат |
| POST | `/api/lead` | заявка/лид → TG+VK |
| POST | `/api/telegram/webhook` | webhook Telegram |
| GET | `/api/admin/stats` | сводка (Bearer ADMIN_TOKEN) |
| GET | `/api/admin/leads` | список лидов |
| GET | `/api/admin/chats` | история чатов |
| PATCH | `/api/admin/leads/{id}` | смена статуса |

## Админ-панель

Страница `/admin` — вход по `ADMIN_TOKEN`, затем дашборд: KPI-карточки
(заявки/чаты за сегодня, среднее время ответа), таблица заявок со сменой статуса,
история чатов. Токен хранится в `localStorage` и шлётся как `Authorization: Bearer`
(для продакшена с HTTPS допустимо; cookie/httpOnly-сессию можно добавить позже).

## Тесты

```bash
cargo test        # unit + интеграционные тесты
cargo clippy --all-targets --all-features
cargo fmt --check
```

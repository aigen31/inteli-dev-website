# inteli-dev.ru

<img src="docs/assets/readme/kiki-header.ru.svg" alt="Кики показывает на браузерную карточку сайта" width="100%">

[English](README.md) · [Русский](README.ru.md) · [中文](README.zh_CN.md)

Личный сайт специалиста по SEO и разработке: витрина услуг, кейсы, блог, CRM-лендинг и ассистент Кики. Один домен и один контур: посетитель читает, задаёт вопрос и оставляет заявку без переходов между сервисами.

Меньше шума и кнопок, больше пользы. Подробные правила — в [`AGENT.md`](AGENT.md).

## Разделы

| Раздел | Путь | Что делает |
|---|---|---|
| Главная | `/` | позиционирование, услуги, кейсы, чат и заявка |
| Услуги | `/services`, `/services/{slug}` | SEO, аудит, разработка и поддержка |
| Проекты | `/projects`, `/projects/{slug}` | кейсы с контекстом и результатом |
| Блог | `/blog`, `/blog/{slug}` | статьи и технические заметки |
| Чат | `/chat` | диалог с Кики, история и быстрые сценарии |
| CRM-лендинг | `/crm` | посадочная страница CRM с кинетической сценой |
| Админка | `/admin/*` | заявки, чаты, статьи, прайс и outbox |
| API | `/api/*` | health, статус, чат, заявки и вебхук Telegram |

## Стек

| Слой | Технология |
|---|---|
| UI | Leptos 0.7.8 (SSR) + ванильный JS |
| HTTP | axum 0.7, tokio, tower |
| Хранилище | SQLite (WAL, sqlx) |
| Контент | встроенный реестр `src/memory/content.rs` |
| LLM | DeepSeek через OpenAI-compatible endpoint |
| Уведомления | Telegram Bot API + VK Messages API |
| Деплой | multi-stage Docker, лёгкий runtime |

## Структура

```text
src/
├── main.rs            boot: конфигурация → слои → сервер
├── lib.rs             библиотека и сборка роутера
├── config.rs          config.toml + переменные окружения
├── api/               HTTP-хендлеры: chat, lead, status, health, admin
├── services/          чат, заявки, статус, SEO, доступность
├── storage/           SQLite: чаты, лиды, статьи, прайс
├── memory/            встроенный контент и страницы услуг
├── llm/               ChatProvider, DeepSeek и промпты
├── notification/      Telegram и VK
├── cache.rs           in-memory кэш и rate limiter
├── utils/             валидация, анонимизация IP
└── ui/                Leptos SSR: home, services, blog, chat, crm, admin
assets/                style.css, main.js, изображения и шрифты
tests/                 интеграционные тесты API и страниц
RULES/                 архитектурные и UI-правила
docs/                  ADR и эксплуатационные заметки
```

## Быстрый старт

### Локально

Нужен Rust 1.85+.

```bash
cp config.toml.example config.toml
cargo run
# → http://localhost:8080
```

Секреты задаются через env (`LLM_API_KEY`, `TELEGRAM_BOT_TOKEN`, `ADMIN_TOKEN`, …) или `config.toml`. Без ключей сайт работает: чат отвечает fallback-ответами, уведомления пропускаются.

### Docker dev

```bash
cp .env.example .env
docker compose -f compose.dev.yaml up -d --build
# → http://localhost:8282
```

### Docker production

Продакшен — канонический `compose.yaml`: приложение за nginx, трафик и TLS через внешний Traefik (сеть `traefik-public`), сертификат Let's Encrypt выпускается автоматически. Контейнеры и тома имеют префикс `inteli-dev-prod`, поэтому production можно держать рядом с dev-стеком.

```bash
cp .env.prod.example .env.prod
docker compose up -d --build
```

Требования к серверу:

- внешняя docker-сеть Traefik: `docker network create traefik-public`;
- A-записи `inteli-dev.ru` и `www` на этот сервер, иначе ACME HTTP-01 не пройдёт.

Конфигурация приложения — `config/config.prod.toml` (монтируется read-only как `/app/config.toml`, секретов в нём нет). Подробности — [`RULES/docker-build.md`](RULES/docker-build.md).

## API

| Метод | Путь | Описание |
|---|---|---|
| GET | `/api/health` | health-check |
| GET | `/api/status` | статус занятости |
| POST | `/api/chat` | AI-чат |
| POST | `/api/lead` | заявка → Telegram + VK |
| POST | `/api/telegram/webhook` | webhook Telegram |
| GET | `/api/admin/stats` | сводка (Bearer `ADMIN_TOKEN`) |
| GET | `/api/admin/leads` | список лидов |
| GET | `/api/admin/chats` | история чатов |
| PATCH | `/api/admin/leads/{id}` | смена статуса |

## Кики и CRM

Кики — общий ассистент сайта и CRM. Ядро живёт в [`inteli-dev-kiki-assistant-kernel-api`](../inteli-dev-kiki-assistant-kernel-api/README.md): оно хранит диалоги, инструменты и маршрутизацию моделей. Сайт отдаёт ядру контекст страницы и принимает tool calls; CRM подключается к тому же ядру.

CRM-лендинг `/crm` использует кинетическую сцену из токенов сайта: Кики, орбитальные кольца, карточки клиентов и задач. Без внешнего фронтенд-фреймворка.

## Тесты

```bash
cargo test
cargo clippy --all-targets --all-features
cargo fmt --check
```

## Документация

- [`AGENT.md`](AGENT.md) — правила разработки, дизайна и деплоя.
- [`RULES/`](RULES/) — архитектура, UI, Docker, безопасность.
- [`docs/ADR.md`](docs/ADR.md) — архитектурные решения.

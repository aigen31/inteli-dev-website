# inteli-dev.ru

<img src="docs/assets/readme/kiki-header.en.svg" alt="Kiki pointing at the inteli-dev.ru browser card" width="100%">

[English](README.md) · [Русский](README.ru.md) · [中文](README.zh_CN.md)

A personal site for SEO and development work: services, cases, blog, the CRM landing page and the Kiki assistant. One domain and one flow: a visitor reads, asks a question and leaves a request without jumping between services.

Less noise, fewer buttons, more useful signal. Detailed rules live in [`AGENT.md`](AGENT.md).

## Sections

| Section | Path | What it does |
|---|---|---|
| Home | `/` | positioning, services, cases, chat and request entry points |
| Services | `/services`, `/services/{slug}` | SEO, audits, development and support |
| Projects | `/projects`, `/projects/{slug}` | case studies with context and results |
| Blog | `/blog`, `/blog/{slug}` | articles and engineering notes |
| Chat | `/chat` | Kiki conversation with history and quick scenarios |
| CRM landing | `/crm` | CRM landing page with the kinetic Kiki scene |
| Admin | `/admin/*` | leads, chats, articles, prices and outbox |
| API | `/api/*` | health, status, chat, leads and the Telegram webhook |

## Stack

| Layer | Technology |
|---|---|
| UI | Leptos 0.7.8 (SSR) + vanilla JS |
| HTTP | axum 0.7, tokio, tower |
| Storage | SQLite (WAL, sqlx) |
| Content | built-in registry `src/memory/content.rs` |
| LLM | DeepSeek via an OpenAI-compatible endpoint |
| Notifications | Telegram Bot API + VK Messages API |
| Delivery | multi-stage Docker, small runtime image |

## Structure

```text
src/
├── main.rs            boot: config → layers → server
├── lib.rs             library and router assembly
├── config.rs          config.toml + environment
├── api/               HTTP handlers: chat, lead, status, health, admin
├── services/          chat, leads, status, SEO, availability
├── storage/           SQLite: chats, leads, articles, prices
├── memory/            built-in content and service pages
├── llm/               ChatProvider, DeepSeek and prompts
├── notification/      Telegram and VK
├── cache.rs           in-memory cache and rate limiter
├── utils/             validation, IP anonymisation
└── ui/                Leptos SSR: home, services, blog, chat, crm, admin
assets/                style.css, main.js, images and fonts
tests/                 API and page integration tests
RULES/                 architecture and UI rules
docs/                  ADRs and operational notes
```

## Quick start

### Local

Rust 1.85+ is required.

```bash
cp config.toml.example config.toml
cargo run
# → http://localhost:8080
```

Secrets come from env (`LLM_API_KEY`, `TELEGRAM_BOT_TOKEN`, `ADMIN_TOKEN`, …) or `config.toml`. Without keys the site still runs: chat falls back to canned replies and notifications are skipped.

### Docker dev

```bash
cp .env.example .env
docker compose -f compose.dev.yaml up -d --build
# → http://localhost:8282
```

### Docker production

Production uses the canonical `compose.yaml`: the app sits behind nginx, traffic and TLS go through an external Traefik network (`traefik-public`), and Let's Encrypt issues the certificate. Containers and volumes use the `inteli-dev-prod` prefix, so production can run next to the dev stack.

```bash
cp .env.prod.example .env.prod
docker compose up -d --build
```

Server requirements:

- an external Traefik Docker network: `docker network create traefik-public`;
- A records for `inteli-dev.ru` and `www` pointing at the server, otherwise ACME HTTP-01 fails.

Application config is `config/config.prod.toml` (mounted read-only as `/app/config.toml`, no secrets). See [`RULES/docker-build.md`](RULES/docker-build.md).

## API

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/health` | health check |
| GET | `/api/status` | availability status |
| POST | `/api/chat` | AI chat |
| POST | `/api/lead` | lead → Telegram + VK |
| POST | `/api/telegram/webhook` | Telegram webhook |
| GET | `/api/admin/stats` | summary (Bearer `ADMIN_TOKEN`) |
| GET | `/api/admin/leads` | lead list |
| GET | `/api/admin/chats` | chat history |
| PATCH | `/api/admin/leads/{id}` | change lead status |

## Kiki and CRM

Kiki is shared by the site and the CRM. The kernel lives in [`inteli-dev-kiki-assistant-kernel-api`](../inteli-dev-kiki-assistant-kernel-api/README.md): it owns conversations, tools and model routing. The site sends page context and accepts tool calls; the CRM connects to the same kernel.

The `/crm` landing page uses a kinetic scene built from the site tokens: Kiki, orbit rings, client and task cards. It does not pull a frontend framework.

## Tests

```bash
cargo test
cargo clippy --all-targets --all-features
cargo fmt --check
```

## Documentation

- [`AGENT.md`](AGENT.md) — development, design and deployment rules.
- [`RULES/`](RULES/) — architecture, UI, Docker, security.
- [`docs/ADR.md`](docs/ADR.md) — architecture decisions.

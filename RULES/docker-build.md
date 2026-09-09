# RULES: Docker-сборка и развёртывание

## 📋 Содержание
- [Принципы сборки](#принципы-сборки)
- [Multi-stage Dockerfile](#multi-stage-dockerfile)
- [docker-compose.yml](#docker-composeyml)
- [Конфигурация окружений](#конфигурация-окружений)
- [Правила оптимизации образа](#правила-оптимизации-образа)
- [Health check и мониторинг](#health-check-и-мониторинг)
- [Развёртывание и обновления](#развёртывание-и-обновления)

---

## Принципы сборки

### Золотые правила Docker для этого проекта
1. **Multi-stage build**: Builder → Runtime, минимум слоёв
2. **Alpine-based runtime**: образ < 50MB (против ~200MB+ для Node.js)
3. **Не копируем .git / dev-зависимости**: только production artifacts
4. **Конфигурация через env vars**: ноль хардкода в образе
5. **Non-root user**: контейнер не запускается от root

### Целевой размер образа: < 50MB
| Этап | Размер | Содержание |
|------|--------|-----------|
| Builder (rust:1.84-slim) | ~2GB | Rust toolchain + Cargo dependencies |
| Intermediate stage (strip) | ~300MB | Стрипнутый бинарник |
| Runtime (alpine:3.20) | **~35MB** | Только бинарник + ca-certificates + tzdata |

---

## Multi-stage Dockerfile

```dockerfile
# ===== STAGE 1: Build =====
# Используем официальный образ Rust с Cargo cache mount
FROM rust:1.84-slim AS builder

WORKDIR /app

# Копируем только зависимости (Cargo.toml + Cargo.lock) — 
# это самый медленный шаг, кэшируется Docker layer
COPY Cargo.toml Cargo.lock ./

# Создаём заглушку для быстрого компиляции зависимостей
RUN mkdir src && echo "fn main(){}" > src/main.rs

# Fetch зависимостей (кэш Docker layer: при изменении только Cargo.toml перекомпилирует)
RUN cargo fetch

# Копируем весь исходный код
COPY . .

# Кэшированные зависимости — если Cargo.toml не менялся, использует кэш
ARG CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu
ENV CARGO_BUILD_TARGET=${CARGO_BUILD_TARGET}

# Компиляция в release режиме
RUN cargo build --release --target ${CARGO_BUILD_TARGET}

# Стрипсим бинарник для уменьшения размера
RUN strip target/${CARGO_BUILD_TARGET}/release/inteli-dev.ru


# ===== STAGE 2: Runtime =====
FROM alpine:3.20 AS runtime

# Минимальные зависимости: ca-certificates (TLS) + tzdata (времяzones)
RUN apk add --no-cache ca-certificates tzdata && rm -rf /var/cache/apk/*

# Создаём non-root пользователя для безопасности
RUN addgroup -S appgroup && adduser -S appuser -G appgroup

WORKDIR /app

# Копируем только бинарник из builder (нечего больше копировать!)
COPY --from=builder --chown=appuser:appgroup /app/target/${CARGO_BUILD_TARGET}/release/inteli-dev.ru ./server

# Ports
EXPOSE 8080

# Health check для Docker monitoring
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -qO- http://localhost:8080/api/health || exit 1

# Переменные окружения по умолчанию
ENV RUST_LOG=info
ENV PORT=8080
ENV CONFIG_PATH=/app/config.toml

# Non-root user
USER appuser

CMD ["/app/server"]
```

### Оптимизированный Dockerfile с BuildKit
```dockerfile
# Dockerfile — версия для CI/CD (включает прогрессивную сборку)
# DOCKER_BUILDKIT=1 docker build -t inteli-dev.ru:latest .

FROM rust:1.84-slim AS builder
WORKDIR /app

# BuildKit mount: кэширует Cargo dependencies между builds
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo fetch

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    strip target/x86_64-unknown-linux-gnu/release/inteli-dev.ru


FROM alpine:3.20 AS runtime
RUN apk add --no-cache ca-certificates tzdata && rm -rf /var/cache/apk/*

ARG APP_VERSION=dev
LABEL org.opencontainers.image.version=${APP_VERSION}
LABEL org.opencontainers.image.description="inteli.dev.ru — SEO specialist portfolio"
LABEL maintainer="admin@inteli.dev.ru"

RUN addgroup -S appgroup && adduser -S appuser -G appgroup
WORKDIR /app

COPY --from=builder --chown=appuser:appgroup \
    /app/target/x86_64-unknown-linux-gnu/release/inteli-dev.ru ./server

EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -qO- http://localhost:8080/api/health || exit 1

ENV RUST_LOG=info PORT=8080
USER appuser
CMD ["/app/server"]
```

---

## docker-compose.yml

### Базовая конфигурация (все сервисы)
```yaml
# docker-compose.yml
version: "3.9"

services:
  # ===== Основное приложение =====
  web:
    build:
      context: .
      dockerfile: Dockerfile
      args:
        - APP_VERSION=${APP_VERSION:-dev}
    container_name: inteli-dev-web
    restart: unless-stopped
    ports:
      - "${WEB_PORT:-8080}:8080"
    environment:
      - RUST_LOG=${RUST_LOG:-info}
      - CONFIG_PATH=/app/config.toml
      # OpenViking
      - OPENVIKING_BASE_URL=http://viking:1933/mcp
      # LLM API
      - LLM_API_KEY=${LLM_API_KEY:?LLM_API_KEY required}
      - LLM_BASE_URL=https://api.deepseek.com/v1
      - LLM_MODEL=deepseek-chat
      # Telegram
      - TELEGRAM_BOT_TOKEN=${TELEGRAM_BOT_TOKEN:?required}
      - TELEGRAM_ADMIN_CHAT_ID=${TELEGRAM_ADMIN_CHAT_ID:?required}
      # VK
      - VK_OAUTH_TOKEN=${VK_OAUTH_TOKEN:?required}
      - VK_ADMIN_USER_ID=${VK_ADMIN_USER_ID:?required}
      # Admin
      - ADMIN_TOKEN=${ADMIN_TOKEN:?required}
      # SQLite
      - DB_PATH=/data/chats.db
    volumes:
      - db-data:/data              # Persistent SQLite data
      - ./config.toml:/app/config.toml:ro  # Read-only config
    depends_on:
      viking:
        condition: service_healthy
      redis:
        condition: service_started
    healthcheck:
      test: ["CMD", "wget", "-qO-", "http://localhost:8080/api/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s

  # ===== OpenViking — семантическая память =====
  viking:
    image: openviking/server:latest
    container_name: inteli-dev-viking
    restart: unless-stopped
    ports:
      - "${VIKING_PORT:-1933}:1933"
    environment:
      - OPENVIKING_DATA_DIR=/data
    volumes:
      - viking-data:/data
      - ./viking-content:/imports:ro  # Initial content import
    healthcheck:
      test: ["CMD", "wget", "-qO-", "http://localhost:1933/health"]
      interval: 15s
      timeout: 5s
      retries: 5
      start_period: 30s

  # ===== Redis — кеширование и сессии =====
  redis:
    image: redis:7-alpine
    container_name: inteli-dev-redis
    restart: unless-stopped
    command: redis-server --appendonly yes --maxmemory 256mb --maxmemory-policy allkeys-lru
    volumes:
      - redis-data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 3s
      retries: 3

volumes:
  db-data:
  viking-data:
  redis-data:
```

### docker-compose.override.yml (для разработки)
```yaml
# docker-compose.override.yml — монтирует исходный код для hot-reload при разработке
version: "3.9"

services:
  web:
    build:
      target: builder  # Только stage 1 для быстрой сборки
    volumes:
      - .:/app         # Mount всего проекта (для dev server)
      - cargo-cache:/usr/local/cargo/registry
      - target-cache:/app/target

volumes:
  cargo-cache:
  target-cache:
```

---

## Конфигурация окружений

### Переменные окружения (все обязательные помечены `:?`)
| Переменная | Описание | Где брать |
|-----------|----------|-----------|
| `LLM_API_KEY` | API ключ для DeepSeek/OpenAI | dashboard.deepseek.com |
| `TELEGRAM_BOT_TOKEN` | Токен бота Telegram | @BotFather |
| `TELEGRAM_ADMIN_CHAT_ID` | ID чата админа в Telegram | bot: /get_updates или @userinfobot |
| `VK_OAUTH_TOKEN` | OAuth токен VK API | developer.vk.com → my app |
| `VK_ADMIN_USER_ID` | ID владельца VK для уведомлений | ваш UID в VK |
| `ADMIN_TOKEN` | Токен авторизации админ-панели | генерировать: `openssl rand -base64 32` |

### Пример .env файла (НЕ коммитить!)
```bash
# .env — локальное окружение, не пушить в Git!
LLM_API_KEY=sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxx
TELEGRAM_BOT_TOKEN=123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11
TELEGRAM_ADMIN_CHAT_ID=-1001234567890
VK_OAUTH_TOKEN=a1b2c3d4e5f6g7h8i9j0
VK_ADMIN_USER_ID=123456789
ADMIN_TOKEN=oPqRsTuVwXyZ1234567890abcdef==
APP_VERSION=1.0.0

# Optional: порты для нестандартных развертываний
WEB_PORT=8080
VIKING_PORT=1933
```

### Production .env (на сервере)
```bash
# /etc/inteli-dev/.env — на продакшен-сервере
LLM_API_KEY=sk-prod-xxxxxxxxxxxxxxxx
TELEGRAM_BOT_TOKEN=prod-bot-token
TELEGRAM_ADMIN_CHAT_ID=-100prod-chat-id
VK_OAUTH_TOKEN=prod-vk-token
VK_ADMIN_USER_ID=prod-user-id
ADMIN_TOKEN=prod-admin-super-secret-token-32chars==
APP_VERSION=1.0.0-prod
RUST_LOG=warn  # Less verbose in production
```

---

## Правила оптимизации образа

### Что НЕ должно попасть в образ
| Элемент | Почему убрать | Как проверить |
|---------|--------------|---------------|
| `.git/` | Не нужен на продакшене | `COPY . .` вместо `COPY --exclude=.git . .` (в Dockerfile) |
| `Cargo.lock` в runtime | Только для build stage | Multi-stage: копируем только бинарник |
| test code | Не нужен в production | `--release` флаг уже excludes dev dependencies |
| `.DS_Store`, `*.swp` | Мусорные файлы | `.dockerignore` |

### .dockerignore
```
# .dockerignore — что НЕ включать в Docker build context
.git/
.github/
.vscode/
.idea/
target/
Cargo.lock
.DS_Store
*.md
viking-content/
config.toml      # Секреты не должны быть в образе
.env
*.sqlite
*.db
```

### Измерение размера образа
```bash
# Построить и проверить размер
docker build -t inteli-dev.ru:latest .

# Показать размер
docker images | grep inteli-dev.ru

# Должно показать ~35-45MB для alpine stage

# Детальная информация по слоям
docker history inteli-dev.ru:latest
```

---

## Health check и мониторинг

### API endpoint для health check
```rust
// api/health.rs — простой health endpoint для Docker/K8s
pub async fn handle_health() -> Result<impl IntoResponse> {
    // Проверяем зависимости
    let db_ok = sqlx::query("SELECT 1").fetch_one(&pool).await.is_ok();
    let redis_ok = redis_client.ping().await.is_ok();
    let viking_ok = viking_client.health_check().await.is_ok();
    
    if !db_ok || !redis_ok {
        // Partial health — сервис работает, но зависимости не OK
        return Ok(Json(serde_json::json!({
            "status": "degraded",
            "database": db_ok,
            "redis": redis_ok,
            "openviking": viking_ok,
            "uptime_seconds": get_uptime_secs(),
        })));
    }
    
    // Full health
    Ok(Json(serde_json::json!({
        "status": "healthy",
        "database": true,
        "redis": true,
        "openviking": viking_ok,
        "uptime_seconds": get_uptime_secs(),
        "version": env!("APP_VERSION", "unknown"),
    })))
}

// Добавляем middleware для отслеживания uptime
static START_TIME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn get_uptime_secs() -> u64 {
    let start = START_TIME.load(std::sync::atomic::Ordering::Relaxed);
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() - start
}

#[tokio::main]
async fn main() {
    START_TIME.store(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        std::sync::atomic::Ordering::Relaxed,
    );
    // ... rest of boot
}
```

### Liveness vs Readiness probes (для Kubernetes в будущем)
```yaml
# Пример для K8s (не для текущего docker-compose, но на будущее)
livenessProbe:
  httpGet:
    path: /api/health
    port: 8080
  initialDelaySeconds: 15
  periodSeconds: 30
  timeoutSeconds: 5
  failureThreshold: 3

readinessProbe:
  httpGet:
    path: /api/health
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 10
  successThreshold: 1
  failureThreshold: 2
```

---

## Развёртывание и обновления

### Команды для развёртывания
```bash
# ===== Разработка (локально) =====
docker compose up --build       # Построить и запустить
docker compose down             # Остановить
docker compose logs -f web      # Просмотр логов приложения
docker compose logs -f viking   # Просмотр логов OpenViking

# ===== Production (на сервере) =====
# 1. Подготовить .env файл на сервере
scp .env user@server:/etc/inteli-dev/.env

# 2. Скопировать docker-compose.yml
scp docker-compose.yml user@server:/opt/inteli-dev/docker-compose.yml

# 3. На сервере:
cd /opt/inteli-dev
docker compose pull             # Скачать новые образы (OpenViking, Redis)
docker compose up -d --pull always  # Пересобрать если нужно, запустить
docker compose ps               # Проверить все сервисы

# ===== Обновление версии приложения =====
# Локально:
docker build -t inteli-dev.ru:latest .
docker tag inteli-dev.ru:latest registry.example.com/inteli-dev.ru:v1.0.0
docker push registry.example.com/inteli-dev.ru:v1.0.0

# На сервере:
ssh user@server "cd /opt/inteli-dev && docker compose pull && docker compose up -d"
```

### Backup стратегии
| Данные | Как бэкапить | Частота | Хранение |
|--------|-------------|---------|----------|
| SQLite (chats.db) | `cp chats.db chats.db.bak && sqlite3 chats.db ".backup chats.db.full"` | Ежедневно | 7 дней локально, архив на S3 30 дней |
| OpenViking data | Копия volumes/viking-data/ | Еженедельно | 4 недели + S3 |
| Config (config.toml) | Git-версия (без секретов!) или encrypted backup | При каждом изменении | Git repo |

### Disaster Recovery
```bash
# Восстановление из бэкапа SQLite
docker exec -it inteli-dev-web sh
cp /data/chats.db.bak /tmp/restore.sql   # export structure
sqlite3 /tmp/chats.db < backup.dump     # import data
cp /tmp/chats.db /data/chats.db          # restore

# Восстановление из образа (если контейнер удалён)
docker compose up -d              # Создёт volumes заново, данные сохранятся в named volumes!

# Полное восстановление (все данные потеряны):
cd /opt/inteli-dev
cp /backup/docker-compose.yml .  # из бэкапа на S3/Git
cp /backup/.env .                # из encrypted backup
docker compose up -d --build
```

---

*См. также: [architecture.md](./architecture.md), [vk-telegram-bridge.md](./vk-telegram-bridge.md)*

# ===== STAGE 1: Build (musl static) =====
# Собираем статический бинарник под musl, чтобы он работал на Alpine-рантайме.
# Полный образ rust: содержит gcc + rustup (для musl-таргета и C-компиляции SQLite).
FROM rust:latest AS builder
WORKDIR /app

# musl-tools даёт musl-gcc (нужен cc-крейту для сборки libsqlite3-sys под musl).
RUN apt-get update && apt-get install -y --no-install-recommends musl-tools \
    && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-unknown-linux-musl

# Кэшируем зависимости: копируем манифест и ставим заглушку main.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src assets && echo "fn main() {}" > src/main.rs && : > assets/style.css
RUN cargo fetch

# Копируем исходники (config.toml/.env/target исключены через .dockerignore).
COPY . .

# Статическая сборка под musl (opt-level=z, lto, strip — см. Cargo.toml [profile.release]).
RUN cargo build --release --target x86_64-unknown-linux-musl
RUN strip target/x86_64-unknown-linux-musl/release/inteli-dev

# ===== STAGE 2: Runtime =====
FROM alpine:3.20 AS runtime

# Минимальные зависимости: TLS-сертификаты + таймзоны + su-exec (drop привилегий).
RUN apk add --no-cache ca-certificates tzdata su-exec && rm -rf /var/cache/apk/*

# Non-root пользователь (P2: безопасность по умолчанию).
RUN addgroup -S appgroup && adduser -S appuser -G appgroup

WORKDIR /app

# Только бинарник + Cargo.toml (нужен Leptos для `get_configuration`).
COPY --from=builder --chown=appuser:appgroup \
    /app/target/x86_64-unknown-linux-musl/release/inteli-dev ./server
COPY --chown=appuser:appgroup Cargo.toml ./

# Entrypoint готовит volume /data (chown для appuser), затем опускается до appuser.
COPY docker/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -qO- http://127.0.0.1:8080/api/health || exit 1

ENV RUST_LOG=info
ENV PORT=8080

# Контейнер стартует от root, чтобы entrypoint мог сделать chown /data;
# само приложение затем запускается su-exec'ом под appuser.
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["/app/server"]

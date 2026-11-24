#!/usr/bin/env bash
# Деплой сайта inteli-dev.ru на прод через SSH (как CRM).
#
# Перекрываем только файлы, которые ведёт этот репозиторий: рабочая копия на
# сервере содержит локальные правки aaPanel и старые WIP-файлы, полный архив
# их бы затёр.
set -euo pipefail

HOST="${DEPLOY_HOST:-root@152.228.118.128}"
APP_DIR="${DEPLOY_APP_DIR:-/www/wwwroot/inteli-dev.ru}"
COMPOSE="docker compose -f compose.yaml -f compose.aapanel.yml"

FILES=(
    src/main.rs
    src/mcp.rs
    src/services/availability.rs
    src/services/mod.rs
    src/services/status.rs
    src/services/chat.rs
    src/ui/admin.rs
    src/ui/mod.rs
    src/ui/shared.rs
    assets/style.css
    compose.yaml
    compose.dev.yaml
)

cd "$(git rev-parse --show-toplevel)"

[ "$(git rev-parse --abbrev-ref HEAD)" = "production" ] || { echo "Нужна ветка production"; exit 1; }
[ -z "$(git status --porcelain)" ] || { echo "Дерево грязное"; exit 1; }

SSH_OPTS=(-F /dev/null -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR)
ASKPASS=""
cleanup() { [ -n "$ASKPASS" ] && rm -f "$ASKPASS" "$ASKPASS.ssh"; }
trap cleanup EXIT

if [ -n "${DEPLOY_SSH_PASSWORD:-}" ]; then
    ASKPASS=$(mktemp)
    printf '#!/bin/sh\nprintf "%%s\\n" %q\n' "$DEPLOY_SSH_PASSWORD" > "$ASKPASS"
    chmod 700 "$ASKPASS"
    printf '#!/bin/sh\nexport SSH_ASKPASS=%s\nexport SSH_ASKPASS_REQUIRE=force\nexec setsid -w ssh %s -o PreferredAuthentications=password -o PubkeyAuthentication=no "$@"\n' "$ASKPASS" "${SSH_OPTS[*]}" > "$ASKPASS.ssh"
    chmod 700 "$ASKPASS.ssh"
    SSH_CMD=(setsid -w env SSH_ASKPASS="$ASKPASS" SSH_ASKPASS_REQUIRE=force ssh "${SSH_OPTS[@]}" -o PreferredAuthentications=password -o PubkeyAuthentication=no)
    GIT_SSH="$ASKPASS.ssh"
else
    SSH_CMD=(ssh "${SSH_OPTS[@]}")
    GIT_SSH="ssh ${SSH_OPTS[*]}"
fi

echo "→ push production в bare-репозиторий"
git remote get-url prod >/dev/null 2>&1 || git remote add prod "$HOST:/root/git/inteli-dev-site.git"
GIT_SSH_COMMAND="$GIT_SSH" git push prod production

echo "→ отправка файлов"
tar czf - "${FILES[@]}" | "${SSH_CMD[@]}" "$HOST" "tar xzf - -C '$APP_DIR'"

echo "→ сборка и перезапуск"
"${SSH_CMD[@]}" "$HOST" "set -e; cd '$APP_DIR'; $COMPOSE build web; $COMPOSE up -d web; for i in \$(seq 1 30); do code=\$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:18002/api/health); [ \"\$code\" = 200 ] && { echo 200; exit 0; }; sleep 2; done; echo \"\$code\"; exit 1"

echo "→ проверка"
"${SSH_CMD[@]}" "$HOST" "curl -s -o /dev/null -w '%{http_code}\\n' http://127.0.0.1:18002/api/health"

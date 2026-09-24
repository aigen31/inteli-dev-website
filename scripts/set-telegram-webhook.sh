#!/usr/bin/env bash
# Регистрация вебхука Telegram для бота настроек сайта.
#
# Telegram должен знать, куда присылать команды владельца. Скрипт запускается
# там, где есть доступ к api.telegram.org (прод-сервер), и берёт токен и секрет
# из того же файла окружения, что и приложение, — так секрет вебхука не может
# разойтись с тем, что проверяет сервер.
#
# Использование:
#   scripts/set-telegram-webhook.sh                       # прод-конфиг по умолчанию
#   ENV_FILE=.env scripts/set-telegram-webhook.sh         # локальный
#   PUBLIC_URL=https://inteli-dev.ru scripts/set-telegram-webhook.sh
#   scripts/set-telegram-webhook.sh --info                # только показать состояние
#   scripts/set-telegram-webhook.sh --delete              # снять вебхук
#
# ⚠️ api.telegram.org может быть недоступен из РФ без прокси — запускайте
#    скрипт с сервера, у которого есть доступ.

set -euo pipefail

ENV_FILE="${ENV_FILE:-.env.prod}"
PUBLIC_URL="${PUBLIC_URL:-https://inteli-dev.ru}"
WEBHOOK_PATH="/api/settings-bot/webhook"

MODE="set"
case "${1:-}" in
  --info)   MODE="info" ;;
  --delete) MODE="delete" ;;
  "")       ;;
  *) echo "Неизвестный аргумент: $1" >&2; exit 2 ;;
esac

if [[ -f "$ENV_FILE" ]]; then
  # shellcheck disable=SC1090
  set -a; source "$ENV_FILE"; set +a
else
  echo "Файл окружения не найден: $ENV_FILE" >&2
  echo "Скопируйте .env.prod.example → .env.prod и заполните секреты." >&2
  exit 1
fi

: "${SETTINGS_BOT_TOKEN:?SETTINGS_BOT_TOKEN не задан в $ENV_FILE}"
: "${SETTINGS_BOT_WEBHOOK_SECRET:?SETTINGS_BOT_WEBHOOK_SECRET не задан в $ENV_FILE}"

API="https://api.telegram.org/bot${SETTINGS_BOT_TOKEN}"

case "$MODE" in
  info)
    curl -fsS "${API}/getWebhookInfo"
    echo
    exit 0
    ;;
  delete)
    curl -fsS "${API}/deleteWebhook" -d '{"drop_pending_updates":false}'
    echo
    exit 0
    ;;
esac

# Убеждаемся, что токен живой, и заодно показываем, какому боту настраиваем.
curl -fsS "${API}/getMe"
echo

# secret_token Telegram будет присылать в заголовке
# X-Telegram-Bot-Api-Secret-Token, который проверяет приложение.
curl -fsS "${API}/setWebhook" \
  --data-urlencode "url=${PUBLIC_URL}${WEBHOOK_PATH}" \
  --data-urlencode "secret_token=${SETTINGS_BOT_WEBHOOK_SECRET}" \
  --data-urlencode "allowed_updates=[\"message\",\"callback_query\"]" \
  --data-urlencode "drop_pending_updates=true"
echo

echo
echo "Готово. Проверка:"
curl -fsS "${API}/getWebhookInfo"
echo

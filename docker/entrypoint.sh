#!/bin/sh
# inteli-dev: входная точка (запускается от root).
#
# Проблема: приложение работает под непривилегированным пользователем `appuser`,
# но volume для SQLite (по умолчанию /data) создаётся Docker от root. Без chown
# SQLite не может создать файл БД -> «unable to open database file».
#
# Здесь мы готовим каталог данных, затем опускаемся до appuser и запускаем сервер.
set -e

DATA_DIR="${DATA_DIR:-/data}"

mkdir -p "$DATA_DIR"
chown -R appuser:appgroup "$DATA_DIR"

exec su-exec appuser:appgroup /app/server "$@"

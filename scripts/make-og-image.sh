#!/usr/bin/env bash
# Пересборка обложки для соцсетей (og:image).
#
# Источник — assets/og-default.svg (правится руками), результат —
# assets/og-default.png, который встраивается в бинарник (`include_bytes!`) и
# отдаётся по /og.png. В соцсети уходит именно PNG: SVG не поддерживают ни
# Telegram, ни VK, ни Twitter.
#
# Использование:
#   scripts/make-og-image.sh
#
# Требуется rsvg-convert (пакет librsvg2-bin / librsvg-tools).
#
# ⚠️ Текст в картинке шрифтом, который есть на машине сборки: подставляемый
#    шрифт перечислен в SVG. Если у вас нет Adwaita Sans, возьмите DejaVu Sans
#    или Liberation Sans — на результат это влияет только визуально.

set -euo pipefail

cd "$(dirname "$0")/.."

SRC="assets/og-default.svg"
OUT="assets/og-default.png"

if ! command -v rsvg-convert >/dev/null 2>&1; then
    echo "rsvg-convert не найден. Установите librsvg2-bin (Debian/Ubuntu) или librsvg-tools (Alpine)." >&2
    exit 1
fi

# 1200×630 — размер, который понимают все площадки (карточка 1.91:1).
rsvg-convert --width 1200 --height 630 --format png --output "$OUT" "$SRC"

echo "Готово: $OUT ($(du -h "$OUT" | cut -f1))"

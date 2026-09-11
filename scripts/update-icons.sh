#!/usr/bin/env bash
# Генерация src/ui/icon.rs из официального пакета Lucide.
#
# Пути иконок НЕ пишутся вручную: скрипт скачивает эталонные SVG из
# `lucide-static` (https://lucide.dev) и подставляет их содержимое в Rust-код.
# Так фигуры гарантированно совпадают с тем, что показывает lucide.dev.
#
# Использование:
#   scripts/update-icons.sh            # зафиксированная версия ниже
#   LUCIDE_VERSION=1.45.0 scripts/update-icons.sh
#
# Лицензия Lucide — ISC (https://lucide.dev/license).

set -euo pipefail

LUCIDE_VERSION="${LUCIDE_VERSION:-1.44.0}"
BASE_URL="https://cdn.jsdelivr.net/npm/lucide-static@${LUCIDE_VERSION}/icons"

# Иконки, используемые на сайте. Список = ключи ICON_PATHS в icon.rs.
#
# Первые шесть — карточки услуг (src/memory/content.rs → Service.icon_name),
# порядок совпадает с порядком услуг:
#   1. Приватные AI-системы      → brain-circuit (нейросеть)
#   2. MCP-серверы и AI Skills   → plug-zap      (интеграция/подключение)
#   3. Голосовые ИИ-боты         → mic           (голос)
#   4. ComfyUI Mass Production   → images        (массовая генерация)
#   5. Fullstack PHP + JS        → code-2        (код)
#   6. DevOps и серверы          → server-cog    (сервер + автоматизация)
#
# Последние три — иконки статуса занятости (services/chat.rs → status_icon_name).
ICONS=(
  brain-circuit
  plug-zap
  mic
  images
  code-2
  server-cog
  circle-check
  settings
  check-circle
)

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/src/ui/icon.rs"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "Lucide v${LUCIDE_VERSION} → ${OUT}"
for icon in "${ICONS[@]}"; do
  curl -fsS --retry 3 "$BASE_URL/$icon.svg" -o "$WORK/$icon.svg" \
    || { echo "не удалось скачать иконку: $icon" >&2; exit 1; }
done

ICONS_JOINED="$(printf '%s\n' "${ICONS[@]}")" \
LUCIDE_VERSION="$LUCIDE_VERSION" \
WORK="$WORK" OUT="$OUT" \
python3 - <<'PY'
import os, re, pathlib

work = pathlib.Path(os.environ["WORK"])
out = pathlib.Path(os.environ["OUT"])
version = os.environ["LUCIDE_VERSION"]
icons = os.environ["ICONS_JOINED"].split()

header = f'''//! Векторные SVG-иконки Lucide (outline, тонкие линии).
//!
//! Файл сгенерирован `scripts/update-icons.sh` из официального пакета
//! `lucide-static@{version}` (https://lucide.dev) — пути вручную не правятся.
//! Лицензия Lucide — ISC: https://lucide.dev/license
//!
//! Каждое значение ниже — точное содержимое `icons/<name>.svg`
//! (дочерние элементы корневого `<svg>`, без обёртки), поэтому фигуры
//! совпадают с эталоном с lucide.dev.

use leptos::prelude::*;
use leptos::svg;

/// Содержимое SVG-тегов для каждой иконки (без обёртки `<svg>…</svg>`).
pub const ICON_PATHS: &[(&str, &str)] = &[
'''

footer = '''];

/// SVG-иконка Lucide (outline, тонкие линии).
///
/// Атрибуты корневого `<svg>` соответствуют официальной разметке Lucide,
/// кроме `stroke-width`: он задан в `1` (тонкие линии в 1px), а не в `2`,
/// как в эталоне. Размер по-прежнему задаётся в CSS (`.card-icon svg`).
#[component]
pub fn LucideIcon(#[prop(into)] name: String) -> impl IntoView {
    let content = ICON_PATHS
        .iter()
        .find(|(k, _)| *k == name.as_str())
        .map(|(_, s)| *s)
        .unwrap_or("");

    svg::svg()
        .attr("xmlns", "http://www.w3.org/2000/svg")
        .attr("viewBox", "0 0 24 24")
        .attr("fill", "none")
        .attr("stroke", "currentColor")
        .attr("stroke-width", "1")
        .attr("stroke-linecap", "round")
        .attr("stroke-linejoin", "round")
        .attr("width", "1em")
        .attr("height", "1em")
        .attr("aria-hidden", "true")
        .inner_html(content)
}
'''

parts = [header]
for name in icons:
    svg = (work / f"{name}.svg").read_text(encoding="utf-8")
    # содержимое = всё после закрывающей скобки корневого <svg ...>
    root_start = svg.index("<svg")
    inner = svg[svg.index(">", root_start) + 1:].rsplit("</svg>", 1)[0]
    tags = re.findall(r"<(?:circle|ellipse|line|path|polygon|polyline|rect)\b[^>]*?/>", inner)
    if not tags:
        raise SystemExit(f"пустой SVG: {name}")
    body = "".join(tags)
    if '"#' in body:
        raise SystemExit(f"конфликт с raw-строкой Rust: {name}")
    parts.append(f'    ("{name}", r#"{body}"#),\n')
    print(f"  {name:16s} {len(tags)} element(s)")
parts.append(footer)

out.write_text("".join(parts), encoding="utf-8")
print(f"готово: {out} ({out.stat().st_size} байт)")
PY

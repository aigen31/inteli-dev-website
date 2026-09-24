#!/usr/bin/env bash
# Генерация src/ui/icon.rs из официальных наборов иконок.
#
# Пути иконок НЕ пишутся вручную: скрипт скачивает эталонные SVG и подставляет
# их содержимое в Rust-код. Так фигуры гарантированно совпадают с оригиналами.
#
# Два набора, потому что они разные по природе:
#   • Lucide (outline, штрих 1px) — интерфейсные иконки: услуги, статус.
#   • Octicons (filled, заливка) — официальный знак GitHub. Брендовые логотипы
#     в Lucide удалили (в 1.44.0 иконки `github` уже нет), а рисовать чужой
#     логотип руками — плохая идея: он должен совпадать с эталоном.
#
# Использование:
#   scripts/update-icons.sh            # зафиксированные версии ниже
#   LUCIDE_VERSION=1.45.0 scripts/update-icons.sh
#   OCTICON_VERSION=19.12.0 scripts/update-icons.sh
#
# Лицензии: Lucide — ISC (https://lucide.dev/license),
#           Octicons — MIT (https://github.com/primer/octicons).

set -euo pipefail

LUCIDE_VERSION="${LUCIDE_VERSION:-1.44.0}"
OCTICON_VERSION="${OCTICON_VERSION:-19.11.0}"
LUCIDE_URL="https://cdn.jsdelivr.net/npm/lucide-static@${LUCIDE_VERSION}/icons"
OCTICON_URL="https://cdn.jsdelivr.net/npm/@primer/octicons@${OCTICON_VERSION}/build/svg"

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

# Брендовые иконки (Octicons). Список = ключи BRAND_ICON_PATHS в icon.rs.
#   mark-github → ссылка на профиль GitHub в шапке сайта.
BRAND_ICONS=(
  mark-github
)

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/src/ui/icon.rs"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "Lucide v${LUCIDE_VERSION} + Octicons v${OCTICON_VERSION} → ${OUT}"

mkdir -p "$WORK/lucide" "$WORK/brand"
for icon in "${ICONS[@]}"; do
  curl -fsS --retry 3 "$LUCIDE_URL/$icon.svg" -o "$WORK/lucide/$icon.svg" \
    || { echo "не удалось скачать иконку Lucide: $icon" >&2; exit 1; }
done
for icon in "${BRAND_ICONS[@]}"; do
  # Octicons кладут размер в имя файла: mark-github-16.svg / mark-github-24.svg.
  # Берём 24 — он ложится на ту же сетку 24×24, что и Lucide.
  curl -fsS --retry 3 "$OCTICON_URL/${icon}-24.svg" -o "$WORK/brand/$icon.svg" \
    || { echo "не удалось скачать иконку Octicons: $icon" >&2; exit 1; }
done

ICONS_JOINED="$(printf '%s\n' "${ICONS[@]}")" \
BRAND_ICONS_JOINED="$(printf '%s\n' "${BRAND_ICONS[@]}")" \
LUCIDE_VERSION="$LUCIDE_VERSION" \
OCTICON_VERSION="$OCTICON_VERSION" \
WORK="$WORK" OUT="$OUT" \
python3 - <<'PY'
import os, re, pathlib

work = pathlib.Path(os.environ["WORK"])
out = pathlib.Path(os.environ["OUT"])
lucide_version = os.environ["LUCIDE_VERSION"]
octicons_version = os.environ["OCTICON_VERSION"]
icons = os.environ["ICONS_JOINED"].split()
brand_icons = os.environ["BRAND_ICONS_JOINED"].split()

# Из SVG берём только фигуры и отбрасываем обёртку <svg>: атрибуты корневого
# тега задаёт компонент, а не сгенерированная строка.
TAG_RE = re.compile(r"<(?:circle|ellipse|line|path|polygon|polyline|rect)\b[^>]*?/>")


def inner_tags(path: pathlib.Path) -> str:
    svg = path.read_text(encoding="utf-8")
    root_start = svg.index("<svg")
    inner = svg[svg.index(">", root_start) + 1:].rsplit("</svg>", 1)[0]
    tags = TAG_RE.findall(inner)
    if not tags:
        raise SystemExit(f"пустой SVG: {path.name}")
    body = "".join(tags)
    if '"#' in body:
        raise SystemExit(f"конфликт с raw-строкой Rust: {path.name}")
    return body


FILE_HEADER = f'''//! Векторные SVG-иконки: Lucide (outline) и Octicons (бренд GitHub).
//!
//! Файл сгенерирован `scripts/update-icons.sh` — пути вручную не правятся.
//!
//! * `ICON_PATHS` — Lucide `{lucide_version}` (https://lucide.dev),
//!   лицензия ISC. Обводка `currentColor`, штрих 1px.
//! * `BRAND_ICON_PATHS` — Octicons `{octicons_version}`
//!   (https://github.com/primer/octicons), лицензия MIT. Заливка
//!   `currentColor`: брендовые логотипы рисуются силуэтом, а не контуром.
//!
//! Наборы разделены, потому что Lucide удалил брендовые иконки (в
//! `{lucide_version}` иконки `github` уже нет), а рисовать чужой логотип
//! руками нельзя — он должен совпадать с официальным.
//!
//! Каждое значение ниже — точное содержимое соответствующего SVG
//! (дочерние элементы корневого `<svg>`, без обёртки).

use leptos::prelude::*;
use leptos::svg;

'''

COMPONENTS = '''
/// SVG-иконка Lucide (outline, тонкие линии).
///
/// Атрибуты корневого `<svg>` соответствуют официальной разметке Lucide,
/// кроме `stroke-width`: он задан в `1` (тонкие линии в 1px), а не в `2`,
/// как в эталоне. Размер по-прежнему задаётся в CSS.
#[component]
pub fn LucideIcon(#[prop(into)] name: String) -> impl IntoView {
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
        .inner_html(lookup(ICON_PATHS, &name))
}

/// SVG-иконка бренда (Octicons): силуэт заливкой, без обводки.
///
/// Отдельный компонент, а не флаг у [`LucideIcon`]: у наборов разные модели
/// отрисовки, и выбирать её по имени иконки значило бы угадывать.
#[component]
pub fn BrandIcon(#[prop(into)] name: String) -> impl IntoView {
    svg::svg()
        .attr("xmlns", "http://www.w3.org/2000/svg")
        .attr("viewBox", "0 0 24 24")
        .attr("fill", "currentColor")
        .attr("stroke", "none")
        .attr("width", "1em")
        .attr("height", "1em")
        .attr("aria-hidden", "true")
        .inner_html(lookup(BRAND_ICON_PATHS, &name))
}

/// Ищет разметку иконки по имени; неизвестное имя даёт пустой `<svg>`.
fn lookup(table: &'static [(&'static str, &'static str)], name: &str) -> &'static str {
    table
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, body)| *body)
        .unwrap_or("")
}
'''

parts = [
    FILE_HEADER,
    "/// Содержимое SVG-тегов Lucide (outline, тонкие линии).\n",
    "pub const ICON_PATHS: &[(&str, &str)] = &[\n",
]
for name in icons:
    parts.append(f'    ("{name}", r#"{inner_tags(work / "lucide" / f"{name}.svg")}"#),\n')
    print(f"  lucide  {name}")
parts.append("];\n\n")

parts.append("/// Содержимое SVG-тегов брендовых иконок (Octicons, заливка).\n")
parts.append("pub const BRAND_ICON_PATHS: &[(&str, &str)] = &[\n")
for name in brand_icons:
    parts.append(f'    ("{name}", r#"{inner_tags(work / "brand" / f"{name}.svg")}"#),\n')
    print(f"  octicon {name}")
parts.append("];\n")
parts.append(COMPONENTS)

out.write_text("".join(parts), encoding="utf-8")
print(f"готово: {out} ({out.stat().st_size} байт)")
PY

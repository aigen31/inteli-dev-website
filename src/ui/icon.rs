//! Векторные SVG-иконки Lucide (outline, тонкие линии).
//!
//! Файл сгенерирован `scripts/update-icons.sh` из официального пакета
//! `lucide-static@1.44.0` (https://lucide.dev) — пути вручную не правятся.
//! Лицензия Lucide — ISC: https://lucide.dev/license
//!
//! Каждое значение ниже — точное содержимое `icons/<name>.svg`
//! (дочерние элементы корневого `<svg>`, без обёртки), поэтому фигуры
//! совпадают с эталоном с lucide.dev.

use leptos::prelude::*;
use leptos::svg;

/// Содержимое SVG-тегов для каждой иконки (без обёртки `<svg>…</svg>`).
pub const ICON_PATHS: &[(&str, &str)] = &[
    ("brain-circuit", r#"<path d="M12 5a3 3 0 1 0-5.997.125 4 4 0 0 0-2.526 5.77 4 4 0 0 0 .556 6.588A4 4 0 1 0 12 18Z" /><path d="M9 13a4.5 4.5 0 0 0 3-4" /><path d="M6.003 5.125A3 3 0 0 0 6.401 6.5" /><path d="M3.477 10.896a4 4 0 0 1 .585-.396" /><path d="M6 18a4 4 0 0 1-1.967-.516" /><path d="M12 13h4" /><path d="M12 18h6a2 2 0 0 1 2 2v1" /><path d="M12 8h8" /><path d="M16 8V5a2 2 0 0 1 2-2" /><circle cx="16" cy="13" r=".5" /><circle cx="18" cy="3" r=".5" /><circle cx="20" cy="21" r=".5" /><circle cx="20" cy="8" r=".5" />"#),
    ("plug-zap", r#"<path d="M6.3 20.3a2.4 2.4 0 0 0 3.4 0L12 18l-6-6-2.3 2.3a2.4 2.4 0 0 0 0 3.4Z" /><path d="m2 22 3-3" /><path d="M7.5 13.5 10 11" /><path d="M10.5 16.5 13 14" /><path d="m18 3-4 4h6l-4 4" />"#),
    ("mic", r#"<path d="M12 19v3" /><path d="M19 10v2a7 7 0 0 1-14 0v-2" /><rect x="9" y="2" width="6" height="13" rx="3" />"#),
    ("images", r#"<path d="m22 11-1.296-1.296a2.4 2.4 0 0 0-3.408 0L11 16" /><path d="M4 8a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2" /><circle cx="13" cy="7" r="1" fill="currentColor" /><rect x="8" y="2" width="14" height="14" rx="2" />"#),
    ("code-2", r#"<path d="m18 16 4-4-4-4" /><path d="m6 8-4 4 4 4" /><path d="m14.5 4-5 16" />"#),
    ("server-cog", r#"<path d="m10.852 14.772-.383.923" /><path d="M13.148 14.772a3 3 0 1 0-2.296-5.544l-.383-.923" /><path d="m13.148 9.228.383-.923" /><path d="m13.53 15.696-.382-.924a3 3 0 1 1-2.296-5.544" /><path d="m14.772 10.852.923-.383" /><path d="m14.772 13.148.923.383" /><path d="M4.5 10H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v4a2 2 0 0 1-2 2h-.5" /><path d="M4.5 14H4a2 2 0 0 0-2 2v4a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-4a2 2 0 0 0-2-2h-.5" /><path d="M6 18h.01" /><path d="M6 6h.01" /><path d="m9.228 10.852-.923-.383" /><path d="m9.228 13.148-.923.383" />"#),
    ("circle-check", r#"<circle cx="12" cy="12" r="10" /><path d="m16 9-5.5 5.5L8 12" />"#),
    ("settings", r#"<path d="M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915" /><circle cx="12" cy="12" r="3" />"#),
    ("check-circle", r#"<path d="M21.801 10A10 10 0 1 1 17 3.335" /><path d="m9 11 3 3L22 4" />"#),
];

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

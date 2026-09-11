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
    ("search", r#"<path d="m21 21-4.34-4.34" /><circle cx="11" cy="11" r="8" />"#),
    ("rocket", r#"<path d="M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5" /><path d="M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09" /><path d="M9 12a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.4 22.4 0 0 1-4 2z" /><path d="M9 12H4s.55-3.03 2-4c1.62-1.08 5 .05 5 .05" />"#),
    ("settings", r#"<path d="M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915" /><circle cx="12" cy="12" r="3" />"#),
    ("pen-tool", r#"<path d="M15.707 21.293a1 1 0 0 1-1.414 0l-1.586-1.586a1 1 0 0 1 0-1.414l5.586-5.586a1 1 0 0 1 1.414 0l1.586 1.586a1 1 0 0 1 0 1.414z" /><path d="m18 13-1.375-6.874a1 1 0 0 0-.746-.776L3.235 2.028a1 1 0 0 0-1.207 1.207L5.35 15.879a1 1 0 0 0 .776.746L13 18" /><path d="m2.3 2.3 7.286 7.286" /><circle cx="11" cy="11" r="2" />"#),
    ("bar-chart-3", r#"<path d="M3 3v16a2 2 0 0 0 2 2h16" /><path d="M18 17V9" /><path d="M13 17V5" /><path d="M8 17v-3" />"#),
    ("user", r#"<path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2" /><circle cx="12" cy="7" r="4" />"#),
    ("briefcase", r#"<path d="M16 20V4a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16" /><rect width="20" height="14" x="2" y="6" rx="2" />"#),
    ("dollar-sign", r#"<line x1="12" x2="12" y1="2" y2="22" /><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" />"#),
    ("mail", r#"<path d="m22 7-8.991 5.727a2 2 0 0 1-2.009 0L2 7" /><rect x="2" y="4" width="20" height="16" rx="2" />"#),
    ("message-square", r#"<path d="M22 17a2 2 0 0 1-2 2H6.828a2 2 0 0 0-1.414.586l-2.202 2.202A.71.71 0 0 1 2 21.286V5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2z" />"#),
    ("send", r#"<path d="M14.536 21.686a.5.5 0 0 0 .937-.024l6.5-19a.496.496 0 0 0-.635-.635l-19 6.5a.5.5 0 0 0-.024.937l7.93 3.18a2 2 0 0 1 1.112 1.11z" /><path d="m21.854 2.147-10.94 10.939" />"#),
    ("check-circle", r#"<path d="M21.801 10A10 10 0 1 1 17 3.335" /><path d="m9 11 3 3L22 4" />"#),
    ("circle-check", r#"<circle cx="12" cy="12" r="10" /><path d="m16 9-5.5 5.5L8 12" />"#),
];

/// SVG-иконка Lucide (outline, тонкие линии).
///
/// Атрибуты корневого `<svg>` соответствуют официальной разметке Lucide;
/// размер и толщина линии задаются в CSS (`.card-icon svg`).
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
        .attr("stroke-width", "2")
        .attr("stroke-linecap", "round")
        .attr("stroke-linejoin", "round")
        .attr("width", "1em")
        .attr("height", "1em")
        .attr("aria-hidden", "true")
        .inner_html(content)
}

//! Векторные SVG-иконки в стиле Lucide (outline, тонкие линии, stroke-width: 2).
//! MIT License — https://github.com/lucide-icons/lucide

use leptos::prelude::*;

/// Карта путей иконок Lucide (outline style).
const ICONS: &[(&str, &str)] = &[
    ("search", r#"<circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>"#),
    ("rocket", r#"<path d="M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z"/><path d="m12 15-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z"/><path d="M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0"/><path d="M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5"/>"#),
    ("settings", r#"<path d="M12 20a8 8 0 1 0 0-16 8 8 0 0 0 0 16Z"/><path d="M9.17 14.83a4 4 0 0 1 5.66 0"/><path d="M9.17 5.17a4 4 0 0 1 5.66 0"/><path d="M12 17v3"/><path d="M12 0v3"/><path d="M4.22 10.59l1.42-1.42"/><path d="M18.36 15.41l1.42-1.42"/><path d="M2 12h3"/><path d="M19 12h3"/><path d="M4.22 13.41l1.42 1.42"/><path d="M18.36 8.59l1.42 1.42"/>"#),
    ("pen-tool", r#"<path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13a5 5 0 0 0-7.21 4.34L9 22l1.66-4.5A5 5 0 0 0 15 13Z"/>"#),
    ("bar-chart-3", r#"<line x1="18" x2="18" y1="20" y2="10"/><line x1="12" x2="12" y1="20" y2="4"/><line x1="6" x2="6" y1="20" y2="14"/>"#),
    ("user", r#"<path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/>"#),
    ("briefcase", r#"<rect width="20" height="14" x="2" y="7" rx="2" ry="2"/><path d="M16 21V5a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16"/>"#),
    ("dollar-sign", r#"<line x1="12" x2="12" y1="2" y2="22"/><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"/>"#),
    ("mail", r#"<rect width="20" height="16" x="2" y="4" rx="2"/><path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7"/>"#),
    ("message-square", r#"<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>"#),
    ("send", r#"<line x1="22" x2="11" y1="2" y2="13"/><polygon points="22,2 15,22 11,13 2,9"/>"#),
    ("check-circle", r#"<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22,4 12,14.01 9,11.01"/>"#),
    ("circle-check", r#"<circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/>"#),
];

/// SVG-иконка в стиле Lucide (outline, тонкие линии).
#[component]
pub fn LucideIcon(#[prop(into)] name: String) -> impl IntoView {
    let icon_path = ICONS.iter().find(|(k, _)| *k == name.as_str()).map_or("", |(_, p)| p);

    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            width="1em"
            height="1em"
            aria-hidden="true"
        >
            <path d={icon_path.to_string()}/>
        </svg>
    }
}

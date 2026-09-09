//! Общие UI-компоненты: кнопки, бейдж статуса, карточки.

use leptos::prelude::*;

use crate::memory::content::{Project, Service, SiteContent};
use crate::services::chat::{label_for_status, status_emoji};

/// Бейдж текущего статуса занятости.
#[component]
pub fn StatusBadge() -> impl IntoView {
    let content = SiteContent::get();
    let status = content.availability.status.clone();
    let emoji = status_emoji(&status).to_string();
    let label = label_for_status(&status).to_string();

    view! {
        <span class=format!("status-badge status-{}", status)>
            <span class="status-dot" aria-hidden="true"></span>
            <span>{emoji} {label}</span>
        </span>
    }
}

/// Primary CTA-кнопка (ссылка).
#[component]
pub fn PrimaryButton(#[prop(into)] text: String, #[prop(into)] href: String) -> impl IntoView {
    view! { <a href=href class="btn btn-primary">{text}</a> }
}

/// Secondary CTA-кнопка (ссылка).
#[component]
pub fn SecondaryButton(#[prop(into)] text: String, #[prop(into)] href: String) -> impl IntoView {
    view! { <a href=href class="btn btn-secondary">{text}</a> }
}

/// Карточка услуги.
#[component]
pub fn ServiceCard(service: Service) -> impl IntoView {
    let price = match (&service.price_from, &service.price_to) {
        (Some(from), Some(to)) => format!("{from} – {to}"),
        (Some(from), None) => format!("от {from}"),
        (None, Some(to)) => format!("до {to}"),
        (None, None) => "по запросу".to_string(),
    };

    view! {
        <article class="card">
            <div class="card-icon" aria-hidden="true">{service.icon}</div>
            <h3 class="card-title">{service.title}</h3>
            <p class="card-text">{service.description}</p>
            <span class="card-price">{price}</span>
        </article>
    }
}

/// Карточка кейса «Было → Стало».
#[component]
pub fn ProjectCard(project: Project) -> impl IntoView {
    view! {
        <article class="card project-card">
            <span class="badge">{project.niche}</span>
            <h3 class="card-title">{project.title}</h3>
            <div class="project-metrics">
                <div class="metric">
                    <span class="metric-label">Было</span>
                    <span class="metric-value metric-before">{project.before}</span>
                </div>
                <div class="metric">
                    <span class="metric-label">Стало</span>
                    <span class="metric-value metric-after">{project.after}</span>
                </div>
            </div>
            <p class="project-result">{project.metric}</p>
        </article>
    }
}

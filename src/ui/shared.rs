//! Общие UI-компоненты: кнопки, бейдж статуса, карточки.

use leptos::prelude::*;

use crate::memory::content::{Project, Service};
use crate::services::status;
use crate::settings::SiteSettings;
use crate::ui::icon::LucideIcon;

/// Бейдж текущего статуса занятости.
///
/// Читает [`SiteSettings`], а не контент: статус правится из Telegram-бота, и
/// бейдж в шапке должен меняться без перезапуска.
///
/// Форма: постоянное подлежащее + короткое состояние («Приём заявок: открыт»).
/// Подлежащее не меняется от состояния к состоянию — иначе бейдж читается как
/// три разных текста, а «ограниченная доступность» без подлежащего непонятна
/// посетителю, который только зашёл. Цвет точки — дублирующий сигнал, не
/// единственный: словесное состояние видно и без наведения, и на телефоне, и
/// скринридеру (через `aria-label`).
///
/// Подсказка с подробностями — **дополнение**: раскрывается наведением (CSS),
/// фокусом с клавиатуры (CSS) и тапом (`initStatusBadge` в `assets/main.js`).
#[component]
pub fn StatusBadge() -> impl IntoView {
    let availability = SiteSettings::availability();
    let p = status::presentation(&availability.status);
    let lines = status::facts(&availability);
    let aria = status::aria_label(&availability);

    view! {
        <button
            type="button"
            class=format!("status-badge status-{}", availability.status)
            aria-expanded="false"
            aria-label=aria
        >
            <span class="status-dot" aria-hidden="true"></span>
            <span class="status-subject">{format!("{}:", p.subject)}</span>
            <span class="status-state">{p.state}</span>
            // `aria-hidden`: тот же текст уже в `aria-label`, дублировать его
            // для скринридера не нужно.
            <span class="status-popover" aria-hidden="true">
                <span class="status-popover-summary">{p.summary}</span>
                {lines
                    .into_iter()
                    .map(|line| view! { <span class="status-popover-line">{line}</span> })
                    .collect::<Vec<_>>()}
            </span>
        </button>
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
            <div class="card-icon" aria-hidden="true"><LucideIcon name=service.icon_name/></div>
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

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

/// Заголовок карточки нужного уровня.
///
/// Уровень — не украшение: краулер нашёл на `/services` и `/projects` пропуск
/// уровня (`<h1>` → `<h3>`) — карточки лежат там прямо в секции страницы. На
/// главной те же карточки стоят внутри секции с `<h2>`, и правильный уровень
/// для них — `<h3>`. Внешний вид задаёт класс, а не тег, поэтому уровень можно
/// менять, не трогая вёрстку и шрифт (`RULES/ui-rules.md` → «Иерархия
/// заголовков»).
///
/// `link` — адрес, на который ведёт заголовок (карточки статей), у карточек
/// услуг и кейсов заголовок не ссылка.
pub fn card_heading(level: u8, class: &'static str, title: String, link: Option<String>) -> AnyView {
    match (level, link) {
        (2, Some(href)) => view! { <h2 class=class><a href=href>{title}</a></h2> }.into_any(),
        (2, None) => view! { <h2 class=class>{title}</h2> }.into_any(),
        (_, Some(href)) => view! { <h3 class=class><a href=href>{title}</a></h3> }.into_any(),
        (_, None) => view! { <h3 class=class>{title}</h3> }.into_any(),
    }
}

/// Карточка услуги.
///
/// `level` — уровень заголовка: 2 на `/services` (карточка — самостоятельный
/// блок страницы), 3 на главной (карточка внутри секции «Услуги»).
#[component]
pub fn ServiceCard(service: Service, #[prop(default = 3)] level: u8) -> impl IntoView {
    let price = match (&service.price_from, &service.price_to) {
        (Some(from), Some(to)) => format!("{from} – {to}"),
        (Some(from), None) => format!("от {from}"),
        (None, Some(to)) => format!("до {to}"),
        (None, None) => "по запросу".to_string(),
    };
    let heading = card_heading(level, "card-title", service.title.clone(), None);

    view! {
        <article class="card">
            <div class="card-icon" aria-hidden="true"><LucideIcon name=service.icon_name/></div>
            {heading}
            <p class="card-text">{service.description}</p>
            <span class="card-price">{price}</span>
        </article>
    }
}

/// Карточка кейса «Было → Стало». `level` — как у [`ServiceCard`].
#[component]
pub fn ProjectCard(project: Project, #[prop(default = 3)] level: u8) -> impl IntoView {
    let heading = card_heading(level, "card-title", project.title.clone(), None);

    view! {
        <article class="card project-card">
            <span class="badge">{project.niche}</span>
            {heading}
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

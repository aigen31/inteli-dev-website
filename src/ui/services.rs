//! Страницы услуг: каталог с фильтром и отдельная страница каждой услуги.
//!
//! Каталог отвечает на вопрос «что и сколько», страница услуги раскрывает
//! результат, состав работы, порядок и ограничения. Текст страницы живёт в
//! `memory::service_pages`, чтобы карточка и подробности не расходились.
//!
//! Категория — свойство услуги (`Service::category`), а не порядок в списке:
//! одна и та же карточка используется и на главной, и здесь. Порядок категорий
//! задан здесь, потому что это вопрос навигации, а не контента.
//!
//! Фильтр работает в браузере: карточки уже в разметке, скрытие — это
//! `hidden` на них. Ни запроса, ни перерисовки, ни потери позиции страницы.

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::api::assets;
use crate::memory::content::SiteContent;
use crate::memory::service_pages;
use crate::ui::shared::{price_label, SecondaryButton, ServiceCard};
use crate::ui::NotFound;

/// Категории в порядке показа: ключ и подпись.
pub const CATEGORIES: &[(&str, &str)] = &[
    ("ai", "ИИ и автоматизация"),
    ("web", "Веб и приложения"),
    ("infra", "Инфраструктура"),
    ("design", "Дизайн и контент"),
    ("quality", "Качество и мелочи"),
];

#[component]
pub fn Services() -> impl IntoView {
    let content = SiteContent::get();
    let services = content.services.clone();
    let total = services.len();

    let groups = CATEGORIES
        .iter()
        .map(|(key, label)| {
            let count = services.iter().filter(|s| s.category == *key).count();
            (key.to_string(), label.to_string(), count)
        })
        .filter(|(_, _, count)| *count > 0)
        .collect::<Vec<_>>();

    // Услуги без категории (контент из OpenViking без поля) не теряются.
    let uncategorized = services.iter().filter(|s| s.category.is_empty()).count();

    view! {
        <section class="section services-page">
            <div class="section-head">
                <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>{"Направления и цены"}</p>
                <h1>Услуги</h1>
            </div>
            <p class="subtitle">{"Что я делаю и сколько это стоит (ориентировочно)."}</p>

            // Фильтр прилипает к шапке: каталог длинный, и возвращаться наверх
            // за категорией каждый раз — лишний жест.
            <div class="service-filter" id="service-filter">
                <div class="service-filter-rail" role="group" aria-label="Категории услуг">
                    <button
                        type="button"
                        class="filter-chip is-active"
                        data-filter="all"
                        aria-pressed="true"
                    >
                        "Все"<span class="filter-count">{total}</span>
                    </button>
                    {groups.into_iter().map(|(key, label, count)| view! {
                        <button
                            type="button"
                            class="filter-chip"
                            data-filter=key
                            aria-pressed="false"
                        >
                            {label}<span class="filter-count">{count}</span>
                        </button>
                    }).collect::<Vec<_>>()}
                    {if uncategorized > 0 {
                        view! {
                            <button type="button" class="filter-chip" data-filter="none" aria-pressed="false">
                                "Прочее"<span class="filter-count">{uncategorized}</span>
                            </button>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}
                </div>
                <p class="service-filter-status" id="service-filter-status" aria-live="polite">
                    {format!("{total} услуг · выберите категорию")}
                </p>
            </div>

            <div class="cards-grid" id="services-list">
                {services.into_iter().map(|s| view! { <ServiceCard service=s level=2/> }).collect::<Vec<_>>()}
            </div>

            // Пустое состояние на случай, если категория осталась без услуг.
            <p class="service-empty" id="services-empty" hidden=true>
                "В этой категории пока пусто — напишите, обсудим задачу отдельно."
            </p>

            <div class="cta-buttons">
                <SecondaryButton text="Обсудить задачу" href="/contact"/>
            </div>
        </section>
    }
}

/// Страница одной услуги: `/services/{slug}`.
///
/// Каталог отвечает на вопрос «что и сколько», эта страница — на вопрос
/// «подходит ли мне и что будет в работе». Контент лежит отдельно от каталога
/// (`memory::service_pages`), чтобы карточка и страница не расходились по
/// смыслу: каталог задаёт цену и короткое описание, страница раскрывает
/// результат, состав, порядок и ограничения.
#[component]
pub fn ServicePage() -> impl IntoView {
    let params = use_params_map();
    let slug = params.read().get("slug").unwrap_or_default();

    let content = SiteContent::get();
    let service = content
        .services
        .iter()
        .find(|s| s.slug == slug)
        .cloned();
    let copy = service_pages::get(&slug);

    let (Some(service), Some(copy)) = (service, copy) else {
        if let Some(options) = use_context::<leptos_axum::ResponseOptions>() {
            options.set_status(axum::http::StatusCode::NOT_FOUND);
        }
        return view! { <NotFound/> }.into_any();
    };

    let index = content
        .services
        .iter()
        .position(|s| s.slug == service.slug)
        .unwrap_or(0)
        + 1;

    let category_label = CATEGORIES
        .iter()
        .find(|(key, _)| *key == service.category)
        .map(|(_, label)| *label)
        .unwrap_or("Услуга");
    let category_word = match service.category.as_str() {
        "ai" => "AI",
        "web" => "WEB",
        "infra" => "INFRA",
        "design" => "DESIGN",
        "quality" => "QUALITY",
        _ => "SERVICE",
    };
    let page_class = format!(
        "section svc-page svc-{}",
        if service.category.is_empty() {
            "other"
        } else {
            service.category.as_str()
        }
    );

    let art = assets::service_art(&service.slug);
    let art_src = art.map(|a| assets::versioned(&assets::service_art_path(a.slug)));
    let art_size = art.map(|a| (a.width, a.height));
    let price = price_label(&service);
    let contact_href = format!("/contact?service={}", service.slug);
    let title = service.title.clone();

    let related: Vec<(String, String, String)> = copy
        .related
        .iter()
        .filter_map(|slug| content.services.iter().find(|s| s.slug == *slug))
        .map(|s| (s.url_path(), s.title.clone(), price_label(s)))
        .collect();
    let cases: Vec<(String, String, String)> = copy
        .projects
        .iter()
        .filter_map(|slug| content.projects.iter().find(|p| p.slug == *slug))
        .map(|p| (p.url_path(), p.title.clone(), p.outcome.clone()))
        .collect();

    let count = content.services.len();
    let position = index.saturating_sub(1);
    let prev = if count > 1 {
        content.services[(position + count - 1) % count].clone()
    } else {
        service.clone()
    };
    let next = if count > 1 {
        content.services[(position + 1) % count].clone()
    } else {
        service.clone()
    };
    let (prev_href, prev_title) = (prev.url_path(), prev.title);
    let (next_href, next_title) = (next.url_path(), next.title);

    view! {
        <article class=page_class>
            <nav class="breadcrumbs" aria-label="Хлебные крошки">
                <a href="/services">"Услуги"</a>
                <span aria-hidden="true">"→"</span>
                <span>{title.clone()}</span>
            </nav>

            <header class="svc-hero">
                <div class="svc-hero-copy">
                    <p class="svc-kicker">
                        <span class="svc-index">{format!("{:02}", index)}</span>
                        <span class="svc-kicker-sep" aria-hidden="true">"/"</span>
                        {copy.kicker}
                    </p>
                    <h1 class="svc-title">{title.clone()}</h1>
                    <p class="svc-lead">{copy.lead}</p>
                    <div class="svc-actions">
                        <a class="btn btn-primary" href=contact_href.clone()>
                            "Обсудить задачу"
                        </a>
                        <a class="btn btn-secondary" href="/chat">"Спросить в чате"</a>
                    </div>
                </div>

                <figure class="svc-art">
                    <span class="svc-art-word" aria-hidden="true">{category_word}</span>
                    {art_src.map(|src| {
                        let (width, height) = art_size.unwrap_or((760, 760));
                        view! {
                            <img
                                class="svc-art-img"
                                src=src
                                alt=format!("Кики: {}", title)
                                width=width
                                height=height
                                decoding="async"
                            />
                        }
                    })}
                    <figcaption class="svc-art-caption">
                        {format!("Кики · {}", category_label)}
                    </figcaption>
                </figure>
            </header>

            <div class="svc-spec" aria-label="Характеристики услуги">
                <div class="svc-spec-item">
                    <span class="svc-spec-label">"Цена"</span>
                    <strong>{price}</strong>
                </div>
                <div class="svc-spec-item">
                    <span class="svc-spec-label">"Формат"</span>
                    <strong>{copy.format}</strong>
                </div>
                <div class="svc-spec-item">
                    <span class="svc-spec-label">"Старт"</span>
                    <strong>"После брифа"</strong>
                </div>
                <div class="svc-spec-item svc-spec-skills">
                    <span class="svc-spec-label">"В работе"</span>
                    <ul>
                        {copy.skills.iter().map(|skill| view! { <li>{*skill}</li> }).collect::<Vec<_>>()}
                    </ul>
                </div>
            </div>

            <section class="svc-section svc-result">
                <div class="svc-section-head">
                    <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>"Результат"</p>
                    <h2>{copy.result}</h2>
                </div>
                <ol class="svc-outcomes">
                    {copy.outcomes.iter().enumerate().map(|(i, text)| view! {
                        <li>
                            <span class="svc-outcome-num">{format!("{:02}", i + 1)}</span>
                            <p>{*text}</p>
                        </li>
                    }).collect::<Vec<_>>()}
                </ol>
            </section>

            <section class="svc-section">
                <div class="svc-section-head">
                    <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>"Состав работы"</p>
                    <h2>"Что входит"</h2>
                </div>
                <ul class="svc-includes">
                    {copy.includes.iter().enumerate().map(|(i, text)| view! {
                        <li>
                            <span class="svc-include-num">{format!("{:02}", i + 1)}</span>
                            <span>{*text}</span>
                        </li>
                    }).collect::<Vec<_>>()}
                </ul>
            </section>

            <section class="svc-section svc-process-section">
                <div class="svc-process-aside">
                    <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>"Порядок"</p>
                    <h2>"Как идёт работа"</h2>
                    <p class="svc-muted">"Ниже обычный порядок. На старте подстраиваю его под задачу."</p>
                </div>
                <ol class="svc-process">
                    {copy.steps.iter().enumerate().map(|(i, (step, text))| view! {
                        <li>
                            <span class="svc-step-num">{format!("{:02}", i + 1)}</span>
                            <div>
                                <h3>{*step}</h3>
                                <p>{*text}</p>
                            </div>
                        </li>
                    }).collect::<Vec<_>>()}
                </ol>
            </section>

            <section class="svc-section">
                <div class="svc-fit">
                    <div class="svc-fit-col svc-fit-yes">
                        <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>"Кому подходит"</p>
                        <ul>
                            {copy.fit.iter().map(|item| view! { <li>{*item}</li> }).collect::<Vec<_>>()}
                        </ul>
                    </div>
                    <div class="svc-fit-col svc-fit-no">
                        <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>"Когда не подходит"</p>
                        <ul>
                            {copy.not_for.iter().map(|item| view! { <li>{*item}</li> }).collect::<Vec<_>>()}
                        </ul>
                    </div>
                </div>
            </section>

            <section class="svc-section">
                <div class="svc-section-head">
                    <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>"До старта"</p>
                    <h2>"Вопросы"</h2>
                </div>
                <div class="svc-faq">
                    {copy.faq.iter().map(|(question, answer)| view! {
                        <details class="svc-faq-item">
                            <summary>
                                {*question}
                                <span class="svc-faq-mark" aria-hidden="true">"+"</span>
                            </summary>
                            <p>{*answer}</p>
                        </details>
                    }).collect::<Vec<_>>()}
                </div>
            </section>

            {if related.is_empty() {
                ().into_any()
            } else {
                view! {
                    <section class="svc-section">
                        <div class="svc-section-head">
                            <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>"Рядом"</p>
                            <h2>"Похожие услуги"</h2>
                        </div>
                        <div class="svc-related">
                            {related.into_iter().map(|(href, name, price)| view! {
                                <a class="svc-related-link" href=href>
                                    <span class="svc-related-price">{price}</span>
                                    <strong>{name}</strong>
                                    <span class="svc-related-arrow" aria-hidden="true">"↗"</span>
                                </a>
                            }).collect::<Vec<_>>()}
                        </div>
                    </section>
                }.into_any()
            }}

            {if cases.is_empty() {
                ().into_any()
            } else {
                view! {
                    <section class="svc-section">
                        <div class="svc-section-head">
                            <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>"Практика"</p>
                            <h2>"Кейсы по теме"</h2>
                        </div>
                        <div class="svc-cases">
                            {cases.into_iter().map(|(href, name, outcome)| view! {
                                <a class="svc-case-link" href=href>
                                    <span class="svc-case-outcome">{outcome}</span>
                                    <strong>{name}</strong>
                                    <span class="svc-related-arrow" aria-hidden="true">"↗"</span>
                                </a>
                            }).collect::<Vec<_>>()}
                        </div>
                    </section>
                }.into_any()
            }}

            <section class="svc-cta">
                <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>"Следующий шаг"</p>
                <h2>"Обсудим вашу задачу"</h2>
                <p class="svc-muted">
                    "Напишите, что нужно сделать и что уже есть. Отвечу с вариантом работы и ориентиром по цене."
                </p>
                <div class="svc-actions">
                    <a class="btn btn-primary" href=contact_href>"Обсудить задачу"</a>
                    <a class="btn btn-secondary" href="/chat">"Спросить в чате"</a>
                </div>
            </section>

            <nav class="svc-nav" aria-label="Другие услуги">
                <a class="svc-nav-link svc-nav-prev" href=prev_href>
                    <span class="svc-nav-label">"Предыдущая"</span>
                    <strong>{prev_title}</strong>
                </a>
                <a class="svc-nav-link svc-nav-next" href=next_href>
                    <span class="svc-nav-label">"Следующая"</span>
                    <strong>{next_title}</strong>
                </a>
            </nav>
        </article>
    }.into_any()
}

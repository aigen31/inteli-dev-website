//! Страница услуг: каталог с фильтром по категориям.
//!
//! Категория — свойство услуги (`Service::category`), а не порядок в списке:
//! одна и та же карточка используется и на главной, и здесь. Порядок категорий
//! задан здесь, потому что это вопрос навигации, а не контента.
//!
//! Фильтр работает в браузере: карточки уже в разметке, скрытие — это
//! `hidden` на них. Ни запроса, ни перерисовки, ни потери позиции страницы.

use leptos::prelude::*;

use crate::memory::content::SiteContent;
use crate::ui::shared::{SecondaryButton, ServiceCard};

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

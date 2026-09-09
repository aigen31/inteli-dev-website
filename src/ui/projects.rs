//! Страница проектов (портфолио).

use leptos::prelude::*;

use crate::memory::content::SiteContent;
use crate::ui::shared::{ProjectCard, SecondaryButton};

#[component]
pub fn Projects() -> impl IntoView {
    let content = SiteContent::get();
    let projects = content.projects.clone();

    view! {
        <section class="section">
            <div class="section-head">
                <h1>Проекты</h1>
            </div>
            <p class="subtitle">{"Конкретные кейсы «до → после» с цифрами."}</p>
            <div class="cards-grid cards-grid-2">
                {projects.into_iter().map(|p| view! { <ProjectCard project=p/> }).collect::<Vec<_>>()}
            </div>
            <div class="cta-buttons">
                <SecondaryButton text="Хочу так же" href="/contact"/>
            </div>
        </section>
    }
}

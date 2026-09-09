//! Главная страница: hero + услуги (превью) + проекты (превью).

use leptos::prelude::*;

use crate::memory::content::SiteContent;
use crate::ui::shared::{PrimaryButton, ProjectCard, SecondaryButton, ServiceCard, StatusBadge};

#[component]
pub fn Home() -> impl IntoView {
    let content = SiteContent::get();
    let profile = content.profile.clone();
    let services_preview = content.services.iter().take(3).cloned().collect::<Vec<_>>();
    let projects_preview = content.projects.iter().take(2).cloned().collect::<Vec<_>>();
    let subtitle = format!(
        "{} · {} лет опыта · {}+ проектов",
        profile.title, profile.experience_years, profile.projects_completed
    );

    view! {
        <section class="hero">
            <div class="hero-content">
                <h1>{profile.name}</h1>
                <p class="subtitle">{subtitle}</p>
                <StatusBadge/>
                <div class="cta-buttons">
                    <PrimaryButton text="💬 Задать вопрос" href="/chat"/>
                    <SecondaryButton text="📩 Оставить заявку" href="/contact"/>
                </div>
            </div>
        </section>

        <section class="section">
            <div class="section-head">
                <h2>Услуги</h2>
                <a class="section-link" href="/services">{"Все услуги →"}</a>
            </div>
            <div class="cards-grid">
                {services_preview.into_iter().map(|s| view! { <ServiceCard service=s/> }).collect::<Vec<_>>()}
            </div>
        </section>

        <section class="section">
            <div class="section-head">
                <h2>Результаты</h2>
                <a class="section-link" href="/projects">{"Все проекты →"}</a>
            </div>
            <div class="cards-grid cards-grid-2">
                {projects_preview.into_iter().map(|p| view! { <ProjectCard project=p/> }).collect::<Vec<_>>()}
            </div>
        </section>

        <section class="section cta-section">
            <h2>Готовы обсудить ваш проект?</h2>
            <p class="subtitle">{"Оставьте заявку — отвечу в течение 24 часов."}</p>
            <div class="cta-buttons">
                <PrimaryButton text="📩 Оставить заявку" href="/contact"/>
                <SecondaryButton text="💬 Спросить в чате" href="/chat"/>
            </div>
        </section>
    }
}

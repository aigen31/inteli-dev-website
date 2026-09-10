//! Главная страница: hero (код-окно + навыки) + услуги + проекты + CTA.
//!
//! Дизайн по мотиву обложки: тёмный индиго, код-терминал слева, крупный
//! заголовок + статус + pills навыков справа, искры и фигурные скобки.

use leptos::prelude::*;

use crate::memory::content::SiteContent;
use crate::ui::shared::{PrimaryButton, ProjectCard, SecondaryButton, ServiceCard, StatusBadge};

/// Декоративное «код-окно» в стиле терминала (как на обложке).
#[component]
fn CodeWindow() -> impl IntoView {
    view! {
        <div class="code-window" aria-hidden="true">
            <div class="code-window-bar">
                <span class="dot dot-red"></span>
                <span class="dot dot-yellow"></span>
                <span class="dot dot-green"></span>
                <span class="code-window-title">{"inteli-dev — терминал"}</span>
            </div>
            <div class="code-window-body">
                <span class="ln"><span class="code-prompt">{"$"}</span> <span class="code-cmd">{"npx create-app"}</span> <span class="code-arg">{"my-site"}</span></span>
                <span class="ln code-out">{"Installing dependencies..."}</span>
                <span class="ln"><span class="code-kw">{"import"}</span> <span class="code-op">{"{ "}</span><span class="code-fn">{"rank"}</span><span class="code-op">{" }"}</span> <span class="code-kw">{"from"}</span> <span class="code-str">{"'seo'"}</span></span>
                <span class="ln"><span class="code-kw">{"const"}</span> <span class="code-var">{"site"}</span> <span class="code-op">{"= "}</span><span class="code-fn">{"new Growth()"}</span></span>
                <span class="ln"><span class="code-var">{"site"}</span><span class="code-op">{"."}</span><span class="code-fn">{"plan"}</span> <span class="code-op">{"= "}</span><span class="code-str">{"'top-3'"}</span></span>
            </div>
        </div>
    }
}

#[component]
pub fn Home() -> impl IntoView {
    let content = SiteContent::get();
    let profile = content.profile.clone();
    let skills = profile.skills.clone();
    let services_preview = content.services.iter().take(3).cloned().collect::<Vec<_>>();
    let projects_preview = content.projects.iter().take(2).cloned().collect::<Vec<_>>();
    let subtitle = format!(
        "{} · {} лет опыта · {}+ проектов",
        profile.title, profile.experience_years, profile.projects_completed
    );

    view! {
        <section class="hero">
            <canvas id="hero-canvas" class="hero-canvas" aria-hidden="true"></canvas>
            <div class="hero-decoration" aria-hidden="true">
                <span class="sparkle sparkle-1"></span>
                <span class="sparkle sparkle-2"></span>
                <span class="braces">{"{...}"}</span>
            </div>

            <div class="hero-grid">
                <CodeWindow/>

                <div class="hero-content">
                    <span class="hero-eyebrow">{"</>"}</span>
                    <h1>{profile.name}</h1>
                    <p class="subtitle">{subtitle}</p>
                    <StatusBadge/>
                    <div class="cta-buttons">
                        <PrimaryButton text="💬 Задать вопрос" href="/chat"/>
                        <SecondaryButton text="📩 Оставить заявку" href="/contact"/>
                    </div>
                    <div class="tech-pills">
                        {skills.into_iter().map(|s| view! { <span class="tech-pill">{s}</span> }).collect::<Vec<_>>()}
                    </div>
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

//! Главная страница: hero (код-окно + навыки) + услуги + проекты + CTA.
//!
//! Дизайн по мотиву обложки: тёмный индиго, код-терминал слева, крупный
//! заголовок + статус + pills навыков справа, искры и фигурные скобки.

use leptos::prelude::*;

use crate::memory::content::SiteContent;
use crate::ui::shared::{PrimaryButton, ProjectCard, SecondaryButton, ServiceCard, StatusBadge};

/// Интерактивный AI-терминал в hero-секции. Клиент может задать вопрос через
/// CLI-интерфейс (как на /chat) и получить ответ от ИИ прямо на главной.
#[component]
pub fn HeroTerminal() -> impl IntoView {
    view! {
        <div class="terminal-window" id="hero-terminal">
            <div class="terminal-bar">
                <span class="dot dot-red"></span>
                <span class="dot dot-yellow"></span>
                <span class="dot dot-green"></span>
                <span class="terminal-title">{"inteli-dev — терминал"}</span>
            </div>

            <div class="terminal-body" id="hero-terminal-output" aria-live="polite"></div>

            <div class="terminal-input-line" id="hero-terminal-input-row">
                <span class="terminal-prompt"><span class="code-prompt">$</span></span>
                <input
                    id="hero-terminal-input"
                    type="text"
                    class="terminal-input"
                    placeholder="Задайте вопрос..."
                    autocomplete="off"
                    aria-label="Введите ваш вопрос"
                />
            </div>

            <div class="terminal-presets" role="group" aria-label="Частые вопросы">
                <button type="button" class="preset-button terminal-preset-btn" data-kind="preset" data-index="0">{"Кто вы?"}</button>
                <button type="button" class="preset-button terminal-preset-btn" data-kind="preset" data-index="1">{"Чем занимаетесь?"}</button>
                <button type="button" class="preset-button terminal-preset-btn" data-kind="preset" data-index="2">{"Сколько стоит?"}</button>
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
            <div class="hero-blob hero-blob-1" aria-hidden="true"></div>
            <div class="hero-blob hero-blob-2" aria-hidden="true"></div>
            <div class="hero-blob hero-blob-3" aria-hidden="true"></div>

            <div class="hero-grid">
                <HeroTerminal/>

                <div class="hero-content">
                    <span class="hero-eyebrow">{"</>"}</span>
                    <h1>{profile.name}</h1>
                    <p class="subtitle">{subtitle}</p>
                    <StatusBadge/>
                    <div class="cta-buttons">
                        <PrimaryButton text="Задать вопрос" href="/chat"/>
                        <SecondaryButton text="Оставить заявку" href="/contact"/>
                    </div>
                    <div class="tech-pills">
                        {skills.into_iter().map(|s| view! { <span class="tech-pill">{s}</span> }).collect::<Vec<_>>()}
                    </div>
                </div>
            </div>
        </section>

        <section class="section section-glow section-services">
            <div class="section-head">
                <h2>Услуги</h2>
                <a class="section-link" href="/services">{"Все услуги →"}</a>
            </div>
            <div class="cards-grid">
                {services_preview.into_iter().map(|s| view! { <ServiceCard service=s/> }).collect::<Vec<_>>()}
            </div>
        </section>

        <section class="section section-glow section-results">
            <div class="section-head">
                <h2>Результаты</h2>
                <a class="section-link" href="/projects">{"Все проекты →"}</a>
            </div>
            <div class="cards-grid cards-grid-2">
                {projects_preview.into_iter().map(|p| view! { <ProjectCard project=p/> }).collect::<Vec<_>>()}
            </div>
        </section>

        <section class="section section-glow section-cta cta-section">
            <h2>Готовы обсудить ваш проект?</h2>
            <p class="subtitle">{"Оставьте заявку — отвечу в течение 24 часов."}</p>
            <div class="cta-buttons">
                <PrimaryButton text="Оставить заявку" href="/contact"/>
                <SecondaryButton text="Спросить в чате" href="/chat"/>
            </div>
        </section>
    }
}

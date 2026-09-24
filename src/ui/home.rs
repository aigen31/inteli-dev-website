//! Главная страница: hero (код-окно + навыки) + услуги + проекты + CTA.
//!
//! Дизайн по мотиву обложки: тёмный индиго, код-терминал слева, крупный
//! заголовок + статус + pills навыков справа, искры и фигурные скобки.

use leptos::prelude::*;

use crate::limits::Limits;
use crate::memory::content::SiteContent;
use crate::ui::blog::LatestArticles;
use crate::ui::github::GitHubStatsSection;
use crate::ui::shared::{PrimaryButton, ProjectCard, SecondaryButton, ServiceCard, StatusBadge};

/// Интерактивный AI-терминал в hero-секции. Клиент может задать вопрос через
/// CLI-интерфейс (как на /chat) и получить ответ от ИИ прямо на главной.
#[component]
pub fn HeroTerminal() -> impl IntoView {
    let max_chars = Limits::get().chat_message_max_chars;
    let chat_hint = Limits::get().chat_hint();
    // Те же кнопки и те же индексы, что на странице /chat: список общий, иначе
    // подпись кнопки на главной могла бы означать другой вопрос.
    let presets = crate::services::chat::hero_presets()
        .map(|p| (p.label.to_string(), p.index))
        .collect::<Vec<_>>();

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
                    maxlength=max_chars
                    aria-describedby="hero-terminal-hint"
                    aria-label="Введите ваш вопрос"
                />
            </div>

            // Лимиты те же, что на /chat: числа берутся из общего Limits.
            <p class="field-hint terminal-hint" id="hero-terminal-hint">
                <span>{chat_hint}</span>
                <span class="char-count" data-counter-for="hero-terminal-input" aria-hidden="true"></span>
            </p>

            <div class="terminal-presets" role="group" aria-label="Частые вопросы">
                {presets.into_iter().map(|(label, index)| view! {
                    <button
                        type="button"
                        class="preset-button terminal-preset-btn"
                        data-kind="preset"
                        data-index=index.to_string()
                    >{label}</button>
                }).collect::<Vec<_>>()}
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

        // Блок «Открытый код»: проверяемый извне аргумент перед финальным CTA.
        // Если GitHub недоступен или блок выключен конфигом — секция не рендерится.
        <GitHubStatsSection/>

        // Последние статьи. Блок сам себя скрывает, пока ничего не опубликовано.
        <LatestArticles/>

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

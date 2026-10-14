//! Главная страница: hero (код-окно + навыки) + услуги + проекты + CTA.
//!
//! Дизайн по мотиву обложки: тёмный индиго, код-терминал слева, крупный
//! заголовок + статус + pills навыков справа, искры и фигурные скобки.

use leptos::prelude::*;

use crate::api::assets;
use crate::limits::Limits;
use crate::memory::content::SiteContent;
use crate::services::goals;
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
    // Версия в адресе обязательна: кадры кэшируются на год (см. assets.rs).
    // Все пять кадров лежат в разметке сразу: смена эмоции — это атрибут, а не
    // загрузка картинки, иначе в момент ответа ведьма мигала бы пустым местом.
    let mascot_frames = assets::MASCOT_FRAMES
        .iter()
        .map(|frame| {
            (
                frame.emotion,
                assets::versioned(&assets::mascot_path(frame.emotion)),
            )
        })
        .collect::<Vec<_>>();

    view! {
        // `data-goal` — та же цель, что у чата на /chat: человек задал вопрос
        // (см. `services::goals`). Терминал отправляет её один раз, после
        // первого ответа сервера.
        <div class="terminal-window" id="hero-terminal" data-goal=goals::CHAT_STARTED>
            <div class="terminal-bar">
                <span class="dot dot-red"></span>
                <span class="dot dot-yellow"></span>
                <span class="dot dot-green"></span>
                <span class="terminal-title">{"inteli-dev — терминал"}</span>
            </div>

            <div class="terminal-body" id="hero-terminal-output" aria-live="polite"></div>

            // Подвал окна: строка ввода, подсказка о лимитах и кнопки. Он лежит
            // на своей, чуть более светлой подложке (см. §7c в style.css) — это
            // и есть та поверхность, на которой сидит маскот: горизонт между
            // лентой и подвалом читается как кромка, а не как линия разметки.
            <div class="terminal-footer">
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

                // Маскот живёт внутри строки ввода, потому что сидит на её
                // верхней черте — границе между лентой сообщений и полем
                // «Задайте вопрос...». Строка ввода тут единственная неподвижная
                // опора: лента сообщений растёт и прокручивается, а черта
                // остаётся на месте.
                // Кадры декоративные: пустой alt у каждого и aria-hidden на
                // контейнере — для скринридера это фон. Эмоцию переключает
                // `data-emotion` (см. `initMascot` в main.js).
                <div class="terminal-mascot" id="hero-mascot" data-emotion="base" aria-hidden="true">
                    {mascot_frames.into_iter().map(|(emotion, src)| view! {
                        <img
                            class="mascot-frame"
                            data-emotion=emotion
                            src=src
                            alt=""
                            width=assets::MASCOT_WIDTH
                            height=assets::MASCOT_HEIGHT
                            decoding="async"
                        />
                    }).collect::<Vec<_>>()}
                </div>
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
                // Маскот — внутри терминала (HeroTerminal): он сидит на черте
                // строки ввода, а не на самом окне.
                <HeroTerminal/>

                <div class="hero-content">
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
            <p class="subtitle">{"Оставьте заявку — отвечу при наличии свободного окна."}</p>
            <div class="cta-buttons">
                <PrimaryButton text="Оставить заявку" href="/contact"/>
                <SecondaryButton text="Спросить в чате" href="/chat"/>
            </div>
        </section>

    }
}

#[cfg(test)]
mod tests {
    use crate::api::assets::STYLE_CSS;

    /// Тела всех CSS-правил с этим селектором — вместе с медиазапросами.
    /// Комментарии выброшены: пояснение рядом с правилом не должно выглядеть как
    /// само правило (в тексте комментария вполне может встретиться `max-height`).
    fn rule_bodies(selector: &str) -> Vec<String> {
        let mut css = String::with_capacity(STYLE_CSS.len());
        let mut tail = STYLE_CSS;
        while let Some(start) = tail.find("/*") {
            css.push_str(&tail[..start]);
            match tail[start + 2..].find("*/") {
                Some(end) => tail = &tail[start + end + 4..],
                None => {
                    tail = "";
                    break;
                }
            }
        }
        css.push_str(tail);

        let mut bodies = Vec::new();
        let mut rest = css.as_str();
        while let Some(pos) = rest.find(selector) {
            let after = &rest[pos + selector.len()..];
            let end = after.find('}').unwrap_or(after.len());
            bodies.push(after[..end].to_string());
            rest = &after[end..];
        }
        bodies
    }

    /// Терминал в hero нельзя ограничивать по высоте: прокручиваться должна
    /// только лента сообщений. Строка ввода, подсказка о лимитах и кнопки — это
    /// вход в чат, и когда окно стояло на `max-height`, их срезало
    /// `overflow: hidden`. Тест текстовый: он ловит возврат `max-height` в CSS,
    /// а не каждую мыслимую причину обрезки.
    #[test]
    fn terminal_window_never_clips_its_input_row_or_presets() {
        let bodies = rule_bodies(".terminal-window {");
        assert!(!bodies.is_empty(), "правило .terminal-window пропало из style.css");
        for body in &bodies {
            assert!(
                !body.contains("max-height"),
                "у .terminal-window не должно быть max-height: вместе с лентой \
                 сообщений окно обрезает подсказку и кнопки. Ограничивать высоту \
                 нужно только у .terminal-body.\nПравило: {{{body}}}"
            );
        }

        let body = rule_bodies(".terminal-body {");
        assert!(
            body.iter().any(|b| b.contains("max-height")),
            "прокручиваться должна лента сообщений: у .terminal-body нет max-height"
        );
    }

    /// Подвал терминала — та поверхность, на которой сидит маскот: у него своя
    /// подложка (светлее ленты) и своя кромка. Без обоих свойств окно снова
    /// становится плоской плашкой, и фигура висит в ней картинкой.
    #[test]
    fn terminal_footer_is_the_surface_the_mascot_sits_on() {
        let bodies = rule_bodies(".terminal-footer {");
        let body = bodies
            .first()
            .unwrap_or_else(|| panic!("правило .terminal-footer пропало из style.css"));
        assert!(
            body.contains("background"),
            "у подвала терминала нет подложки — не будет и глубины: {{{body}}}"
        );
        assert!(
            body.contains("border-top"),
            "кромка подвала — та линия, на которой сидит маскот: в правиле её нет, {{{body}}}"
        );
        assert!(
            !rule_bodies(".terminal-input-line {")
                .iter()
                .any(|b| b.contains("border-top")),
            "черта принадлежит границе «лента — подвал», а не строке ввода: \
             вернувшийся border-top у .terminal-input-line даст две кромки"
        );
    }

    /// Тень маскота — тёмная и направленная. Светлый ореол в цвет primary
    /// возвращает ровное свечение вокруг силуэта, а оно читается как левитация:
    /// у сидящей фигуры тень уходит вниз-вправо, а не расходится во все стороны.
    #[test]
    fn mascot_shadow_is_dark_and_directional() {
        let bodies = rule_bodies(".mascot-frame {");
        let body = bodies
            .first()
            .unwrap_or_else(|| panic!("правило .mascot-frame пропало из style.css"));
        assert!(
            body.contains("drop-shadow"),
            "тень маскота пропала: {{{body}}}"
        );
        assert!(
            !body.contains("139, 127, 245"),
            "ореол в цвет primary вернулся — он читается как свечение вокруг \
             фигуры, то есть как полёт: {{{body}}}"
        );
        assert!(
            !body.contains("drop-shadow(0 0"),
            "тень без смещения расходится вокруг силуэта: направление (вниз-вправо) \
             и есть то, что делает её тенью: {{{body}}}"
        );
    }

    /// У каждого кадра своя поправка посадки: опора контейнера (-80%) верна
    /// только для base, у кадров с поднятыми коленями кромка без поправки идёт
    /// по бёдрам. Новый кадр без `--seat-nudge` молча повиснет над кромкой.
    #[test]
    fn every_mascot_frame_sits_on_the_ledge() {
        for frame in crate::api::assets::MASCOT_FRAMES {
            let selector = format!(".mascot-frame[data-emotion=\"{}\"] {{", frame.emotion);
            let bodies = rule_bodies(&selector);
            assert!(
                bodies.iter().any(|b| b.contains("--seat-nudge")),
                "у кадра {} нет поправки посадки (--seat-nudge): он повиснет над \
                 кромкой подвала",
                frame.emotion
            );
        }
    }
}

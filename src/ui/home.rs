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

/// Сколько кейсов крутить в слайдере на главной.
///
/// Четыре — компромисс: два слайда не дают повода листать, а все восемь
/// превращали бы главную в копию `/projects` (ровно та дубликация, из-за которой
/// страницы портфолио плохо индексируются).
const SLIDER_COUNT: usize = 4;

/// Интерактивный AI-терминал в hero-секции. Клиент может задать вопрос через
/// CLI-интерфейс (как на /chat) и получить ответ от ИИ прямо на главной.
#[component]
pub fn HeroTerminal() -> impl IntoView {
    let max_chars = Limits::get().chat_message_max_chars;
    let chat_hint = Limits::get().chat_hint();
    // Контакты нужны командам терминала (`/mail`, `/tg`): они не ходят на
    // сервер, поэтому данные приходят вместе с разметкой.
    let contacts = SiteContent::get().profile.contacts.clone();
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
        <div
            class="terminal-window"
            id="hero-terminal"
            data-goal=goals::CHAT_STARTED
            data-email=contacts.email.clone()
            data-telegram=contacts.telegram.clone()
        >
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
                <span>"Команды: /help · /brief · /audit"</span>
                <span class="terminal-hint-sep" aria-hidden="true">"·"</span>
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
    // Слайдер кейсов: четыре штуки — достаточно, чтобы листать, и меньше, чем
    // весь портфолио на `/projects` (дублировать его целиком на главной незачем).
    let slides = content
        .projects
        .iter()
        .take(SLIDER_COUNT)
        .cloned()
        .collect::<Vec<_>>();
    let slide_count = slides.len();
    // Цифры вынесены в отдельную моно-строку: в подзаголовке они тонули в
    // тексте, а в новом макете это метрика, а не часть описания.
    let metrics = format!(
        "{} лет опыта · {}+ проектов",
        profile.experience_years, profile.projects_completed
    );
    let hero_art = crate::api::assets::versioned(crate::api::assets::HERO_ART_PATH);
    let hero_art_w = crate::api::assets::HERO_ART_WIDTH;
    let hero_art_h = crate::api::assets::HERO_ART_HEIGHT;
    view! {
        // Hero — обложка во всю ширину: слева текст, справа иллюстрация
        // (Кики в левитации). Картинка уходит под левую колонку градиентной
        // шторкой, поэтому заголовку не нужна подложка и он не спорит с
        // фигурой. Пятна и сетка переехали на уровень страницы (`.site-bg`).
        <section class="hero">
            <div class="hero-art" aria-hidden="true">
                <img
                    class="hero-art-img"
                    src=hero_art
                    alt=""
                    width=hero_art_w
                    height=hero_art_h
                    fetchpriority="high"
                    decoding="async"
                />
            </div>

            <div class="hero-inner">
                <div class="hero-content">
                    <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>{profile.location.clone()}</p>
                    <h1 class="hero-title">{profile.name}</h1>
                    <p class="hero-role">{profile.title.clone()}</p>
                    <p class="hero-metrics mono">{metrics}</p>
                    <div class="hero-actions">
                        <StatusBadge/>
                        <div class="cta-buttons">
                            <PrimaryButton text="Задать вопрос" href="/chat"/>
                            <SecondaryButton text="Оставить заявку" href="/contact"/>
                        </div>
                    </div>
                    <ul class="tech-pills" aria-label="Специализация">
                        {skills.into_iter().map(|s| view! { <li class="tech-pill">{s}</li> }).collect::<Vec<_>>()}
                    </ul>
                </div>
            </div>
        </section>

        // Терминал вынесен из hero в отдельную секцию: в обложке он спорил с
        // иллюстрацией за правую половину, а здесь у него своя полоса и
        // собственный заголовок. Маскот остаётся внутри окна (см. §7c/d).
        <section class="section section-terminal" id="terminal">
            <div class="terminal-layout">
                <div class="terminal-intro">
                    <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>{"Терминал"}</p>
                    <h2>Спросите Кики</h2>
                    <p class="section-note">
                        "Она знает услуги, сроки и стек. Отвечает сразу — без формы и ожидания письма."
                    </p>
                    <ul class="terminal-facts">
                        <li><span class="terminal-fact-num mono">01</span>{"Ответ по услугам, срокам и стеку"}</li>
                        <li><span class="terminal-fact-num mono">02</span>{"Три ответа в час, без регистрации"}</li>
                        <li><span class="terminal-fact-num mono">03</span>{"Не хватит ответа — оставите заявку"}</li>
                    </ul>
                </div>
                <HeroTerminal/>
            </div>
        </section>

        <section class="section section-services">
            <div class="section-head">
                <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>{"01 — Направления"}</p>
                <h2>Услуги</h2>
                <a class="section-link" href="/services">{"Все услуги"}</a>
            </div>
            <div class="cards-grid">
                {services_preview.into_iter().map(|s| view! { <ServiceCard service=s/> }).collect::<Vec<_>>()}
            </div>
        </section>

        <section class="section section-results">
            <div class="section-head">
                <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>{"02 — Кейсы"}</p>
                <h2>Результаты</h2>
                <div class="section-actions">
                    // Стрелки приходят выключенными: без скрипта они бы ничего не
                    // делали, а включённые `initProjectSlider` — работают. Свайп и
                    // колесо работают и без них.
                    <button
                        type="button"
                        class="slider-arrow"
                        data-slider="prev"
                        aria-label="Предыдущий кейс"
                        disabled
                    >
                        <span aria-hidden="true">"←"</span>
                    </button>
                    <button
                        type="button"
                        class="slider-arrow"
                        data-slider="next"
                        aria-label="Следующий кейс"
                        disabled
                    >
                        <span aria-hidden="true">"→"</span>
                    </button>
                    <a class="section-link" href="/projects">{"Все проекты"}</a>
                </div>
            </div>

            // Лента со снапом: одна карточка на экран. Прокручивается сама —
            // стрелки и точки только помогают, но без них лента не ломается.
            <div
                class="project-slider"
                id="project-slider"
                role="region"
                aria-label="Кейсы"
                tabindex="0"
            >
                <ul class="project-slider-track">
                    {slides
                        .into_iter()
                        .enumerate()
                        .map(|(i, p)| view! {
                            <li class="project-slide" id=format!("project-slide-{}", i + 1)>
                                <ProjectCard project=p/>
                            </li>
                        })
                        .collect::<Vec<_>>()}
                </ul>
            </div>

            // Точки — ссылки, а не кнопки: без скрипта переход по якорю всё равно
            // прокручивает ленту к нужному слайду.
            <div class="project-slider-dots" role="group" aria-label="Выбор кейса">
                {(1..=slide_count)
                    .map(|i| view! {
                        <a
                            class="slider-dot"
                            href=format!("#project-slide-{i}")
                            aria-label=format!("Кейс {i} из {slide_count}")
                        ></a>
                    })
                    .collect::<Vec<_>>()}
            </div>
        </section>

        // Блок «Открытый код»: проверяемый извне аргумент перед финальным CTA.
        // Если GitHub недоступен или блок выключен конфигом — секция не рендерится.
        <GitHubStatsSection/>

        // Последние статьи. Блок сам себя скрывает, пока ничего не опубликовано.
        <LatestArticles/>

        <section class="section section-cta cta-section">
            <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>{"04 — Контакт"}</p>
            <h2>Готовы обсудить ваш проект?</h2>
            <p class="section-note">{"Оставьте заявку — отвечу при наличии свободного окна."}</p>
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

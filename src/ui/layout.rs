//! Layout: header (навигация) + main + footer.
//!
//! Здесь же живёт плавающая кнопка чата: она нужна на всех страницах, а Layout —
//! единственное место, которое эти страницы оборачивает.

use leptos::prelude::*;
use leptos_router::location::RequestUrl;

use crate::api::assets;
use crate::limits::Limits;
use crate::memory::content::SiteContent;
use crate::services::github;
use crate::services::goals;
use crate::ui::beget::BegetLogo;
use crate::ui::icon::{BrandIcon, LucideIcon};
use crate::ui::shared::StatusBadge;

/// Ссылка на профиль Habr.
///
/// Адрес — константа, а не поле контента: у Habr нет профиля, который сайт мог
/// бы перечитать при старте (в отличие от GitHub, чей логин приходит из API), а
/// сам профиль — витрина статей, не часть данных сайта.
const HABR_URL: &str = "https://habr.com/ru/users/intelidev/";

/// Путь текущего запроса.
///
/// `None` вне leptos_axum (юнит-тест, чужой потребитель) — тогда виджет чата
/// показывается: страница-исключение одна, и пропадать из-за отсутствия
/// контекста он не должен.
fn request_path() -> Option<String> {
    let url = use_context::<RequestUrl>()?;
    let parsed = url.parse().ok()?;
    Some(parsed.path().to_string())
}

#[component]
pub fn Layout(children: Children) -> impl IntoView {
    let content = SiteContent::get();
    let name = content.profile.name.clone();
    let telegram = content.profile.contacts.telegram.clone();
    let email = content.profile.contacts.email.clone();
    let tagline = format!("{name} — Fullstack и приватные AI-системы");

    // Ссылка на GitHub — из глобального профиля (публикуется при старте).
    // Никак не связана со снимком статистики, поэтому не исчезает при
    // недоступном GitHub API.
    let github_link = github::profile().map(|p| {
        let label = format!("Профиль {} на GitHub", p.login);
        (p.url, label, p.login)
    });

    // Плавающий чат: кнопка в правом нижнем углу и компактная панель к ней.
    // Есть на каждой странице, кроме `/chat`: там чат занимает всю страницу, и
    // вторая кнопка «Спросить» над ним дублировала бы поле ввода.
    let chat_widget = request_path().as_deref() != Some("/chat");
    // Кнопки в компактной панели — то же подмножество, что в терминале на
    // главной: список общий с /chat, поэтому подпись и индекс кнопки не могут
    // разойтись с тем, как их обрабатывает сервер.
    let presets_fab = crate::services::chat::hero_presets()
        .map(|p| (p.label.to_string(), p.kind.to_string(), p.index))
        .collect::<Vec<_>>();
    // Лимит поля тот же, что у терминала и /chat: серверная валидация одна.
    let max_chars = Limits::get().chat_message_max_chars;

    view! {
        <a href="#main" class="skip-link">Перейти к содержимому</a>

        <header class="navbar">
            <nav class="navbar-inner" aria-label="Основная навигация">
                <a href="/" class="navbar-brand">{name}</a>

                // Бургер виден только на мобильном (см. RULES/ui-rules.md:
                // «Navbar: <768px — hamburger menu, лого слева»). Состояние
                // переключает initMobileNav() в assets/main.js.
                <button
                    type="button"
                    class="navbar-toggle"
                    id="navbar-toggle"
                    aria-controls="navbar-menu"
                    aria-expanded="false"
                    aria-label="Открыть меню"
                >
                    <span class="navbar-toggle-box" aria-hidden="true">
                        <span class="navbar-toggle-bar"></span>
                        <span class="navbar-toggle-bar"></span>
                        <span class="navbar-toggle-bar"></span>
                    </span>
                </button>

                <div class="navbar-menu" id="navbar-menu">
                    <div class="navbar-links">
                        <a href="/">Главная</a>
                        <a href="/services">Услуги</a>
                        <a href="/projects">Проекты</a>
                        <a href="/blog">Блог</a>
                        <a href="/chat">Чат</a>
                        <a href="/contact">Контакты</a>
                    </div>

                    // Внешние профили уехали в подвал: в шапке они спорили за
                    // место с навигацией, а статус занятости остаётся крайним
                    // справа — он главный сигнал шапки.
                    <StatusBadge/>
                </div>
            </nav>
        </header>

        <main id="main" class="main">{children()}</main>

        <footer class="footer">
            <div class="footer-inner">
                <div class="footer-col">
                    <p>{tagline}</p>
                    <p class="footer-contacts">
                        // Те же цели, что и на странице контактов
                        // (см. `services::goals`): подвал есть на каждой
                        // странице, и переход в мессенджер оттуда — такой же
                        // интерес к контакту, как с `/contact`.
                        <a href=format!("https://t.me/{}", telegram.trim_start_matches('@')) data-goal=goals::TG_CLICK>{telegram.clone()}</a>
                        <span aria-hidden="true">{"·"}</span>
                        <a href=format!("mailto:{}", email) data-goal=goals::EMAIL_CLICK>{email.clone()}</a>
                    </p>

                    // Внешние профили. `rel="me"` — стандартный способ
                    // подтвердить, что профиль принадлежит владельцу сайта.
                    // Кнопки того же веса, что были в шапке: круглая форма,
                    // подпись — в `aria-label`.
                    <div class="footer-social">
                        {github_link.map(|(url, label, login)| view! {
                            <a
                                class="social-link"
                                href=url
                                target="_blank"
                                rel="me noopener noreferrer"
                                title=format!("GitHub — {login}")
                                aria-label=label
                            >
                                <BrandIcon name="mark-github"/>
                            </a>
                        })}

                        // Знак Habr — отдельный файл (`assets/habr.svg`), а не
                        // инлайн-SVG: официальная фигура весит ~16 КБ, и в
                        // разметке каждой страницы она удваивала бы вес
                        // документа.
                        <a
                            class="social-link"
                            href=HABR_URL
                            target="_blank"
                            rel="me noopener noreferrer"
                            title="Профиль на Habr"
                            aria-label="Профиль на Habr"
                        >
                            <img
                                class="social-icon"
                                src=assets::versioned(assets::HABR_ICON_PATH)
                                alt=""
                                width="20"
                                height="20"
                                decoding="async"
                            />
                        </a>
                    </div>
                </div>

                <div class="footer-col footer-col-beget">
                    <span class="footer-beget-label">{"работает на"}</span>
                    <a
                        class="footer-beget-logo"
                        href="https://beget.com/p1627316"
                        target="_blank"
                        rel="noopener noreferrer nofollow"
                        aria-label="Хостинг Beget"
                    >
                        <BegetLogo/>
                    </a>
                </div>
            </div>
        </footer>

        // Плавающий чат: кнопка в правом нижнем углу и компактная панель к ней.
        // Панель лежит в разметке всегда и выключена атрибутом `hidden`: так она
        // стоит ноль на первой отрисовке и не создаётся в JS. Видимость кнопки
        // (терминал прокручен / чат уже открывали) и вся переписка — в
        // `initFloatingChat` (`assets/main.js`).
        {chat_widget.then(|| view! {
            <div class="chat-fab" id="chat-fab" hidden>
                <section
                    class="chat-fab-panel"
                    id="chat-fab-panel"
                    hidden
                    aria-label="Чат с AI-ассистентом"
                    data-goal=goals::CHAT_STARTED
                >
                    <header class="chat-fab-head">
                        <span class="chat-fab-title">{"AI-ассистент"}</span>
                        <button
                            type="button"
                            class="chat-fab-close"
                            id="chat-fab-close"
                            aria-label="Свернуть чат"
                        >
                            <span aria-hidden="true">{"×"}</span>
                        </button>
                    </header>

                    <div class="chat-fab-body">
                        <p class="chat-fab-intro" id="chat-fab-intro">
                            "Спросите о задачах, услугах и сроках — отвечу прямо здесь."
                        </p>
                        <div class="chat-fab-presets" id="chat-fab-presets" role="group" aria-label="Частые вопросы">
                            {presets_fab.into_iter().map(|(label, kind, index)| view! {
                                <button
                                    type="button"
                                    class="preset-button chat-fab-preset"
                                    data-kind=kind
                                    data-index=index.to_string()
                                >{label}</button>
                            }).collect::<Vec<_>>()}
                        </div>
                        <div class="chat-fab-messages" id="chat-fab-messages" role="log" aria-live="polite"></div>
                    </div>

                    <div class="chat-fab-input-row">
                        <input
                            id="chat-fab-input"
                            type="text"
                            placeholder="Ваш вопрос..."
                            autocomplete="off"
                            maxlength=max_chars
                            aria-label="Ваш вопрос"
                        />
                        <button id="chat-fab-send" type="button" class="btn btn-primary">"Отправить"</button>
                    </div>
                </section>

                <button
                    type="button"
                    class="chat-fab-toggle"
                    id="chat-fab-toggle"
                    aria-expanded="false"
                    aria-controls="chat-fab-panel"
                    aria-label="Открыть чат"
                >
                    <span class="chat-fab-toggle-icon" aria-hidden="true"><LucideIcon name="message-circle"/></span>
                    <span class="chat-fab-toggle-label">"Спросить"</span>
                </button>
            </div>
        })}
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::icon::{BRAND_ICON_PATHS, ICON_PATHS};

    /// Регенерация `icon.rs` могла не найти иконку — тогда в шапке рисовался бы
    /// пустой `<svg>`. Фиксируем, что разметка знака GitHub на месте.
    #[test]
    fn github_brand_mark_has_geometry() {
        let body = BRAND_ICON_PATHS
            .iter()
            .find(|(name, _)| *name == "mark-github")
            .map(|(_, body)| *body);

        assert!(
            body.is_some_and(|b| b.contains("<path")),
            "иконка mark-github отсутствует или пуста в BRAND_ICON_PATHS"
        );
    }

    /// Оба набора нужны страницам: Lucide — карточки услуг и статус,
    /// Octicons — ссылка на GitHub.
    #[test]
    fn both_icon_sets_are_present() {
        assert!(!ICON_PATHS.is_empty());
        assert!(!BRAND_ICON_PATHS.is_empty());
    }
}

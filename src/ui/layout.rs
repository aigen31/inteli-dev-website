//! Layout: header (навигация) + main + footer.

use leptos::prelude::*;

use crate::memory::content::SiteContent;
use crate::services::github;
use crate::ui::beget::BegetLogo;
use crate::ui::icon::BrandIcon;
use crate::ui::shared::StatusBadge;

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

                    // Ссылка на профиль GitHub. `rel="me"` — стандартный способ
                    // подтвердить, что профиль принадлежит владельцу сайта.
                    {github_link.map(|(url, label, login)| view! {
                        <a
                            class="navbar-social-link"
                            href=url
                            target="_blank"
                            rel="me noopener noreferrer"
                            title=format!("GitHub — {login}")
                            aria-label=label
                        >
                            <BrandIcon name="mark-github"/>
                        </a>
                    })}

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
                        <a href=format!("https://t.me/{}", telegram.trim_start_matches('@'))>{telegram.clone()}</a>
                        <span aria-hidden="true">{"·"}</span>
                        <a href=format!("mailto:{}", email)>{email.clone()}</a>
                    </p>
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

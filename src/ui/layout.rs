//! Layout: header (навигация) + main + footer.

use leptos::prelude::*;

use crate::memory::content::SiteContent;
use crate::ui::beget::BegetLogo;
use crate::ui::shared::StatusBadge;

#[component]
pub fn Layout(children: Children) -> impl IntoView {
    let content = SiteContent::get();
    let name = content.profile.name.clone();
    let telegram = content.profile.contacts.telegram.clone();
    let email = content.profile.contacts.email.clone();
    let tagline = format!("{name} — Fullstack и приватные AI-системы");

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
                        <a href="/chat">Чат</a>
                        <a href="/contact">Контакты</a>
                    </div>
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

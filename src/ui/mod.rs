//! UI Layer — Leptos SSR-компоненты (без WASM/hydration).
//!
//! Интерактивность (чат, анимации) — на ванильном JS в `assets/main.js`.
//! Контент читается из `SiteContent` (OpenViking → fallback) синхронно, т.к.
//! SSR рендерится на сервере.

pub mod admin;
pub mod chat;
pub mod contact;
pub mod home;
pub mod icon;
pub mod layout;
pub mod projects;
pub mod services;
pub mod shared;

use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use layout::Layout;

/// Корневой компонент приложения.
#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Layout>
                <Routes fallback=|| view! { <NotFound/> }>
                    <Route path=path!("/") view=home::Home/>
                    <Route path=path!("/services") view=services::Services/>
                    <Route path=path!("/projects") view=projects::Projects/>
                    <Route path=path!("/chat") view=chat::Chat/>
                    <Route path=path!("/contact") view=contact::Contact/>
                    <Route path=path!("/admin") view=admin::Admin/>
                </Routes>
            </Layout>
        </Router>
    }
}

/// Страница 404.
#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <section class="hero">
            <div class="hero-content">
                <h1>404</h1>
                <p class="subtitle">Страница не найдена</p>
                <a class="btn btn-primary" href="/">На главную</a>
            </div>
        </section>
    }
}

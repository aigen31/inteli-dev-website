//! Страница услуг.

use leptos::prelude::*;

use crate::memory::content::SiteContent;
use crate::ui::shared::{SecondaryButton, ServiceCard};

#[component]
pub fn Services() -> impl IntoView {
    let content = SiteContent::get();
    let services = content.services.clone();

    view! {
        <section class="section">
            <div class="section-head">
                <h1>Услуги</h1>
            </div>
            <p class="subtitle">Что я делаю и сколько это стоит (ориентировочно).</p>
            <div class="cards-grid">
                {services.into_iter().map(|s| view! { <ServiceCard service=s/> }).collect::<Vec<_>>()}
            </div>
            <div class="cta-buttons">
                <SecondaryButton text="Обсудить задачу" href="/contact"/>
            </div>
        </section>
    }
}

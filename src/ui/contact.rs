//! Страница контактов: форма заявки + прямые контакты.
//!
//! Форма отправляется ванильным JS (`assets/main.js`) на POST /api/lead.

use leptos::prelude::*;

use crate::memory::content::SiteContent;

#[component]
pub fn Contact() -> impl IntoView {
    let content = SiteContent::get();
    let contacts = content.profile.contacts.clone();

    view! {
        <section class="section">
            <div class="section-head">
                <h1>Контакты</h1>
            </div>
            <p class="subtitle">{"Опишите проект — отвечу в течение 24 часов."}</p>

            <div class="contact-grid">
                <form id="lead-form" class="form" method="post" action="/api/lead">
                    <div class="form-field">
                        <label for="name">Имя</label>
                        <input id="name" name="name" type="text" required placeholder="Как к вам обращаться"/>
                    </div>
                    <div class="form-field">
                        <label for="email">Email</label>
                        <input id="email" name="email" type="email" placeholder="you@example.com"/>
                    </div>
                    <div class="form-field">
                        <label for="phone">Телефон</label>
                        <input id="phone" name="phone" type="tel" placeholder="+7 (___) ___-__-__"/>
                    </div>
                    <div class="form-field">
                        <label for="message">О проекте</label>
                        <textarea id="message" name="message" required rows="4"
                            placeholder="Например: интернет-магазин, ~500 страниц, нужен рост заявок"></textarea>
                    </div>
                    <input type="hidden" name="source" value="form"/>
                    <button type="submit" class="btn btn-primary">Отправить заявку</button>
                    <p id="lead-form-status" class="form-status" role="status" aria-live="polite"></p>
                </form>

                <aside class="contacts-aside">
                    <h2>Напрямую</h2>
                    <ul class="contacts-list">
                        <li><a href=format!("https://t.me/{}", contacts.telegram.trim_start_matches('@'))>{contacts.telegram.clone()}</a></li>
                        <li><a href=format!("mailto:{}", contacts.email)>{contacts.email.clone()}</a></li>
                        <li><a href=format!("https://{}", contacts.vk)>{contacts.vk.clone()}</a></li>
                    </ul>
                </aside>
            </div>
        </section>
    }
}

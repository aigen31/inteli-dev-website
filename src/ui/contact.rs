//! Страница контактов: форма заявки + прямые контакты.
//!
//! Форма отправляется ванильным JS (`assets/main.js`) на POST /api/lead.

use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::limits::Limits;
use crate::memory::content::SiteContent;
use crate::services::goals;
use crate::utils::validator::EMAIL_MAX_CHARS;

#[component]
pub fn Contact() -> impl IntoView {
    let content = SiteContent::get();
    let contacts = content.profile.contacts.clone();
    let limits = Limits::get();

    // Ссылка со страницы услуги приносит слаг в query. Название услуги
    // подставляется в текст заявки, а source хранит конкретную услугу: так
    // видно, какая страница привела лид, без отдельной таблицы.
    let query = use_query_map();
    let service_slug = query.read().get("service").unwrap_or_default();
    let service = content
        .services
        .iter()
        .find(|s| s.slug == service_slug)
        .cloned();
    let service_title = service
        .as_ref()
        .map(|s| s.title.clone())
        .unwrap_or_default();
    let prefill = service
        .as_ref()
        .map(|s| format!("Интересует услуга: {}.\n\nКоротко о задаче: ", s.title))
        .unwrap_or_default();
    let source = service
        .as_ref()
        .map(|s| format!("service:{}", s.slug))
        .unwrap_or_else(|| "form".to_string());

    view! {
        <section class="section">
            <div class="section-head">
                <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>{"Связь"}</p>
                <h1>Контакты</h1>
            </div>
            <p class="subtitle">{"Опишите проект: задача, сроки, бюджет — этого достаточно, чтобы начать."}</p>
            {if service_title.is_empty() {
                ().into_any()
            } else {
                view! {
                    <p class="contact-service-note">
                        "Заявка по услуге: "
                        <strong>{service_title.clone()}</strong>
                    </p>
                }.into_any()
            }}

            <div class="contact-grid">
                // `data-goal` — цель аналитики (см. `services::goals`):
                // срабатывает успешная отправка, а не нажатие кнопки, поэтому
                // в статистике нет заявок, которые не дошли до сервера.
                <form id="lead-form" class="form" method="post" action="/api/lead"
                    data-goal=goals::LEAD_SUBMITTED>
                    <div class="form-field">
                        <label for="name">Имя</label>
                        <input id="name" name="name" type="text" required
                            maxlength=limits.lead_name_max_chars
                            placeholder="Как к вам обращаться"/>
                        <span class="char-count" data-counter-for="name" aria-hidden="true"></span>
                    </div>
                    <div class="form-field">
                        <label for="email">Email</label>
                        <input id="email" name="email" type="email"
                            maxlength=EMAIL_MAX_CHARS
                            placeholder="you@example.com"/>
                    </div>
                    <div class="form-field">
                        <label for="phone">Телефон</label>
                        <input id="phone" name="phone" type="tel"
                            maxlength=limits.lead_phone_max_chars
                            placeholder="+7 (___) ___-__-__"/>
                        <span class="char-count" data-counter-for="phone" aria-hidden="true"></span>
                    </div>
                    <div class="form-field">
                        <label for="message">О проекте</label>
                        <textarea id="message" name="message" required rows="4"
                            maxlength=limits.lead_message_max_chars
                            placeholder="Например: интернет-магазин, ~500 страниц, нужен рост заявок">{prefill.clone()}</textarea>
                        <span class="char-count" data-counter-for="message" aria-hidden="true"></span>
                    </div>
                    <input type="hidden" name="source" value=source/>
                    <button type="submit" class="btn btn-primary">Отправить заявку</button>
                    <p id="lead-form-status" class="form-status" role="status" aria-live="polite"></p>
                </form>

                <aside class="contacts-aside">
                    <h2>Напрямую</h2>
                    <ul class="contacts-list">
                        <li><a href=format!("https://t.me/{}", contacts.telegram.trim_start_matches('@')) data-goal=goals::TG_CLICK>{contacts.telegram.clone()}</a></li>
                        <li><a href=format!("mailto:{}", contacts.email) data-goal=goals::EMAIL_CLICK>{contacts.email.clone()}</a></li>
                        <li><a href=format!("https://{}", contacts.vk) data-goal=goals::VK_CLICK>{contacts.vk.clone()}</a></li>
                    </ul>
                </aside>
            </div>
        </section>
    }
}

//! Страница AI-чата.
//!
//! Рендерит preset-кнопки, область сообщений и поле ввода. Вся интерактивность
//! (отправка в POST /api/chat, рендер ответов) — в `assets/main.js`.

use leptos::prelude::*;

/// Шесть preset-кнопок (см. chatbot.md).
const PRESETS: [(&str, &str, usize); 6] = [
    ("👤 Кто вы?", "preset", 0),
    ("💼 Чем можете помочь?", "preset", 1),
    ("💰 Сколько стоит?", "preset", 2),
    ("🔍 Проанализируйте мой сайт", "analysis", 3),
    ("🟢 Когда свободны?", "availability", 4),
    ("📩 Оставить заявку", "lead_request", 5),
];

#[component]
pub fn Chat() -> impl IntoView {
    let presets = PRESETS
        .iter()
        .map(|(label, kind, index)| (label.to_string(), kind.to_string(), *index))
        .collect::<Vec<_>>();

    view! {
        <section class="section chat-section">
            <div class="section-head">
                <h1>Задайте вопрос</h1>
            </div>
            <p class="subtitle">Отвечу на вопросы об услугах, ценах и сроках.</p>

            <div class="chat-container" id="chat-app">
                <div class="preset-buttons" role="group" aria-label="Частые вопросы">
                    {presets.into_iter().map(|(label, kind, index)| view! {
                        <button
                            type="button"
                            class="preset-button"
                            data-kind=kind
                            data-index=index.to_string()
                        >{label}</button>
                    }).collect::<Vec<_>>()}
                </div>

                <div class="messages-area" id="chat-messages" role="log" aria-live="polite"></div>

                <div class="chat-input-row">
                    <input id="chat-input" type="text" placeholder="Ваш вопрос..." autocomplete="off"/>
                    <button id="chat-send" type="button" class="btn btn-primary">Отправить</button>
                </div>

                <a href="/contact" class="floating-cta">{"📩 Оставить заявку"}</a>
            </div>
        </section>
    }
}

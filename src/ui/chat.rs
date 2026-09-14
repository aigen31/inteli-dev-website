//! Страница AI-чата.
//!
//! Рендерит preset-кнопки, область сообщений и поле ввода. Вся интерактивность
//! (отправка в POST /api/chat, рендер ответов) — в `assets/main.js`.

use leptos::prelude::*;

use crate::limits::Limits;

/// Preset-кнопки: кто, чем, локальный инференс, анализ, доступность, заявка.
const PRESETS: [(&str, &str, usize); 6] = [
    ("Кто вы?", "preset", 0),
    ("Чем занимаетесь?", "preset", 1),
    ("Локальный инференс — что это?", "preset", 2),
    ("Проанализируйте мой сайт", "analysis", 3),
    ("Когда свободны?", "availability", 4),
    ("Оставить заявку", "lead_request", 5),
];

#[component]
pub fn Chat() -> impl IntoView {
    let presets = PRESETS
        .iter()
        .map(|(label, kind, index)| (label.to_string(), kind.to_string(), *index))
        .collect::<Vec<_>>();

    let max_chars = Limits::get().chat_message_max_chars;
    let chat_hint = Limits::get().chat_hint();

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
                    <input
                        id="chat-input"
                        type="text"
                        placeholder="Ваш вопрос..."
                        autocomplete="off"
                        maxlength=max_chars
                        aria-describedby="chat-hint"
                    />
                    <button id="chat-send" type="button" class="btn btn-primary">Отправить</button>
                </div>

                // Предупреждение о лимитах: числа берутся из тех же Limits, что
                // и серверная валидация, поэтому подсказка не может соврать.
                <p class="field-hint" id="chat-hint">
                    <span>{chat_hint}</span>
                    <span class="char-count" data-counter-for="chat-input" aria-hidden="true"></span>
                </p>

                <a href="/contact" class="floating-cta">{"Оставить заявку"}</a>
            </div>
        </section>
    }
}

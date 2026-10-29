//! Страница AI-чата.
//!
//! Рендерит готовые вопросы, область сообщений и поле ввода. Вся
//! интерактивность (отправка в POST /api/chat, рендер ответов) — в
//! `assets/main.js`.
//!
//! Готовые вопросы — те же термины из [`crate::services::glossary`], что и в
//! терминале на главной: слаг кнопки один на все поверхности, поэтому подпись
//! и ответ не могут разойтись. Старые кнопки-пресеты («Кто вы», «Сколько
//! стоит») убраны: они дублировали обложку и цены.

use leptos::prelude::*;

use crate::limits::Limits;
use crate::services::goals;
use crate::ui::shared::{GlossaryLinks, QuickActions};

#[component]
pub fn Chat() -> impl IntoView {
    let max_chars = Limits::get().chat_message_max_chars;
    let chat_hint = Limits::get().chat_hint();

    view! {
        <section class="section chat-section">
            <div class="section-head">
                <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>{"Диалог с Кики"}</p>
                <h1>Задайте вопрос</h1>
            </div>
            <p class="subtitle">{"Отвечу о проектах, статьях и ценах. Или выберите готовый вопрос ниже."}</p>

            // `data-goal` — цель «диалог начат» (см. `services::goals`).
            // initChat() отправляет её один раз, после первого ответа сервера.
            <div class="chat-container" id="chat-app" data-goal=goals::CHAT_STARTED>
                // Термины объясняются готовым текстом: без запроса к модели.
                <GlossaryLinks/>

                <div class="messages-area" id="chat-messages" role="log" aria-live="polite"></div>

                <div class="chat-input-row">
                    <input
                        id="chat-input"
                        type="text"
                        placeholder="Ваш вопрос..."
                        autocomplete="off"
                        maxlength=max_chars
                        aria-describedby="chat-hint"
                        // Видимой подписи у поля нет (дизайн чата), поэтому
                        // доступное имя задаётся здесь — как в hero-терминале
                        // на главной и как требует RULES/ui-rules.md.
                        aria-label="Ваш вопрос"
                    />
                    <button id="chat-send" type="button" class="btn btn-primary">Отправить</button>
                </div>

                // Предупреждение о лимитах: числа берутся из тех же Limits, что
                // и серверная валидация, поэтому подсказка не может соврать.
                <p class="field-hint" id="chat-hint">
                    <span>{chat_hint}</span>
                    <span class="char-count" data-counter-for="chat-input" aria-hidden="true"></span>
                </p>

                <QuickActions/>
            </div>
        </section>
    }
}

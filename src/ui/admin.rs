//! Админ-панель (SSR-оболочка + ванильный JS).
//!
//! Дизайн-принцип (admin-panel.md): «админка функциональная, а не красивая».
//! Логика загрузки данных, смены статусов и редактирования статей — в
//! `assets/main.js` (initAdmin), данные приходят с `/api/admin/*` с
//! `Authorization: Bearer <ADMIN_TOKEN>`.
//!
//! Разметка здесь задаёт только «скелет»: таблицы и списки целиком собирает JS,
//! чтобы не дублировать шаблоны на Rust и на JavaScript.

use leptos::prelude::*;

#[component]
pub fn Admin() -> impl IntoView {
    view! {
        <section class="section admin-section">
            <h1>Админ-панель</h1>
            <p class="subtitle">{"Заявки, чаты, статьи блога и события кросспостинга."}</p>

            <div id="admin-app">
                // Вход по токену.
                <div id="admin-login" class="admin-login">
                    <label for="admin-token">Токен доступа</label>
                    <div class="admin-login-row">
                        <input id="admin-token" type="password" autocomplete="off"
                            placeholder="Введите ADMIN_TOKEN"/>
                        <button id="admin-login-btn" type="button" class="btn btn-primary">Войти</button>
                    </div>
                    <p id="admin-login-status" class="form-status" role="status" aria-live="polite"></p>
                </div>

                // Дашборд (скрыт до входа).
                <div id="admin-dashboard" class="admin-dashboard" hidden>
                    <div class="kpi-cards" id="admin-kpi"></div>

                    <h2>Заявки</h2>
                    <div class="table-wrap">
                        <table id="admin-leads-table" class="admin-table" aria-label="Заявки"></table>
                    </div>

                    <h2>Чаты</h2>
                    <div class="table-wrap">
                        <table id="admin-chats-table" class="admin-table" aria-label="История чатов"></table>
                    </div>

                    <h2>Статьи блога</h2>
                    <div class="admin-toolbar">
                        <div class="admin-filters" id="admin-article-filters" role="group" aria-label="Фильтр по статусу">
                            <button type="button" class="filter-btn is-active" data-status="">"Все"</button>
                            <button type="button" class="filter-btn" data-status="draft">"Черновики"</button>
                            <button type="button" class="filter-btn" data-status="published">"Опубликованные"</button>
                            <button type="button" class="filter-btn" data-status="archived">"Архив"</button>
                        </div>
                        <button type="button" id="article-new-btn" class="btn btn-primary">"Новая статья"</button>
                    </div>
                    <p class="admin-hint" id="admin-articles-summary"></p>

                    <div class="table-wrap">
                        <table id="admin-articles-table" class="admin-table" aria-label="Статьи"></table>
                    </div>

                    // Редактор: скрыт, пока не выбрана статья или не нажата
                    // «Новая статья».
                    <div id="article-editor" class="admin-editor" hidden>
                        <h3 id="article-editor-heading">"Новая статья"</h3>

                        <div class="admin-editor-grid">
                            <div class="admin-editor-form">
                                <label for="article-title">"Заголовок"</label>
                                <input id="article-title" type="text" maxlength="200"/>

                                <label for="article-slug">"Адрес (slug)"</label>
                                <input id="article-slug" type="text" maxlength="120"
                                    placeholder="пусто — сгенерируется из заголовка"/>
                                <p class="field-hint">"Латиница, цифры и дефисы. Меняйте только осознанно: адрес попадает в ссылки и RSS."</p>

                                <label for="article-summary">"Краткое описание"</label>
                                <textarea id="article-summary" rows="2" maxlength="500"></textarea>
                                <p class="field-hint">"Показывается в списке блога, RSS и в описании для поисковиков."</p>

                                <label for="article-tags">"Теги"</label>
                                <input id="article-tags" type="text" placeholder="rust, ai, mcp"/>
                                <p class="field-hint">"Через запятую, не больше 10."</p>

                                <label for="article-cover">"Обложка"</label>
                                <input id="article-cover" type="text" placeholder="https://… или /img/…"/>

                                <label for="article-canonical">"Канонический адрес"</label>
                                <input id="article-canonical" type="text" placeholder="если статья уже опубликована в другом месте"/>

                                <label for="article-status">"Статус"</label>
                                <select id="article-status">
                                    <option value="draft">"Черновик — не виден на сайте"</option>
                                    <option value="published">"Опубликована — видна в блоге"</option>
                                    <option value="archived">"В архиве — снята с публикации"</option>
                                </select>

                                <label for="article-body">"Текст (Markdown)"</label>
                                <textarea id="article-body" class="admin-editor-body" rows="18"
                                    placeholder="# Заголовок&#10;&#10;Текст статьи…"></textarea>

                                <div class="admin-editor-actions">
                                    <button type="button" id="article-save-btn" class="btn btn-primary">"Сохранить"</button>
                                    <button type="button" id="article-preview-btn" class="btn btn-secondary">"Предпросмотр"</button>
                                    <button type="button" id="article-cancel-btn" class="btn btn-secondary">"Закрыть"</button>
                                </div>
                                <p id="article-editor-status" class="form-status" role="status" aria-live="polite"></p>
                            </div>

                            <div class="admin-editor-preview">
                                <div class="admin-editor-preview-head">
                                    <strong>"Предпросмотр"</strong>
                                    <span id="article-preview-meta" class="admin-hint"></span>
                                </div>
                                <div id="article-preview" class="prose admin-preview-body">
                                    <p class="admin-hint">"Нажмите «Предпросмотр», чтобы увидеть, как статья будет выглядеть на сайте."</p>
                                </div>
                            </div>
                        </div>
                    </div>

                    <h2>Кросспостинг</h2>
                    <p class="admin-hint">
                        "События, которые забирает n8n. Публикация статьи создаёт событие в той же транзакции, что и запись статьи."
                    </p>
                    <p class="admin-hint" id="admin-outbox-summary"></p>
                    <div class="table-wrap">
                        <table id="admin-outbox-table" class="admin-table" aria-label="События кросспостинга"></table>
                    </div>
                </div>
            </div>
        </section>
    }
}

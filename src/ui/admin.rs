//! Админ-панель (SSR-оболочка + ванильный JS).
//!
//! Дизайн-принцип (admin-panel.md): «админка функциональная, а не красивая».
//! Логика загрузки данных и смены статусов — в `assets/main.js` (initAdmin),
//! данные приходят с `/api/admin/*` с `Authorization: Bearer <ADMIN_TOKEN>`.

use leptos::prelude::*;

#[component]
pub fn Admin() -> impl IntoView {
    view! {
        <section class="section admin-section">
            <h1>Админ-панель</h1>
            <p class="subtitle">{"Просмотр заявок, чатов и метрик."}</p>

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
                </div>
            </div>
        </section>
    }
}

//! Админ-панель: оболочка с разделами (SSR) + ванильный JS.
//!
//! # Почему разделы — это адреса, а не вкладки
//!
//! Вкладки на одной странице удобны автору, а не тому, кто ими пользуется:
//! ссылку на заявки не отправить, обновление страницы возвращает в начало, а
//! «назад» уводит на сайт. Поэтому у каждого раздела свой адрес
//! (`/admin/leads`, `/admin/articles`, …), а оболочка — одна: сайдбар со
//! списком разделов и заголовок страницы. Данные грузит `initAdmin` в
//! `assets/main.js`; раздел просит только свои ручки и не тянет чужие таблицы.
//!
//! # Что здесь решено так, а не иначе
//!
//! * **Вход — отдельная карточка, а не раздел.** Токен живёт в
//!   `localStorage`; пока его нет, сайдбар бесполезен (все ручки ответят 401).
//!   Поэтому до входа видна только форма, после — оболочка.
//! * **Заголовок страницы — название раздела.** В админке важно, где ты, а не
//!   что это «админка»: это написано в сайдбаре один раз.
//! * **Действие раздела — в шапке страницы.** «Новая статья» стоит там же, где
//!   заголовок списка: так это устроено в большинстве админок, и искать кнопку
//!   рядом с фильтрами не приходится.
//! * **Разметку таблиц собирает JS.** Шаблон на Rust и шаблон на JavaScript
//!   разошлись бы на первой же правке колонок; сервер задаёт только места
//!   (`#admin-leads-table` и т.п.).

use leptos::prelude::*;

use crate::services::settings_bot::{MAX_PROJECTS, TEXT_FIELD_MAX_CHARS};

/// Раздел админки.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Overview,
    Leads,
    Chats,
    Articles,
    Outbox,
    Prices,
    Settings,
}

impl Section {
    /// Адрес раздела.
    fn path(self) -> &'static str {
        match self {
            Section::Overview => "/admin",
            Section::Leads => "/admin/leads",
            Section::Chats => "/admin/chats",
            Section::Articles => "/admin/articles",
            Section::Outbox => "/admin/outbox",
            Section::Prices => "/admin/prices",
            Section::Settings => "/admin/settings",
        }
    }

    /// Заголовок страницы и пункта меню.
    fn title(self) -> &'static str {
        match self {
            Section::Overview => "Обзор",
            Section::Leads => "Заявки",
            Section::Chats => "Чаты",
            Section::Articles => "Статьи",
            Section::Outbox => "Кросспостинг",
            Section::Prices => "Прайс",
            Section::Settings => "Приём заявок",
        }
    }

    /// Подпись под заголовком: одна строка о том, что здесь делают.
    fn subtitle(self) -> &'static str {
        match self {
            Section::Overview => "Что происходит на сайте сегодня и куда идти дальше.",
            Section::Leads => "Заявки с сайта: контакт, сообщение и статус обработки.",
            Section::Chats => "Вопросы в AI-чате, ответы ассистента и время отклика.",
            Section::Articles => "Черновики, публикации и архив статей блога.",
            Section::Outbox => {
                "События публикации, которые забирает n8n для кросспостинга."
            }
            Section::Prices => "Цены, которые называет ассистент и показывает сайт.",
            Section::Settings => "Бейдж в шапке сайта — его же меняет Telegram-бот.",
        }
    }
}

/// Обзор админки.
#[component]
pub fn AdminOverview() -> impl IntoView {
    page(Section::Overview)
}

/// Раздел «Заявки».
#[component]
pub fn AdminLeads() -> impl IntoView {
    page(Section::Leads)
}

/// Раздел «Чаты».
#[component]
pub fn AdminChats() -> impl IntoView {
    page(Section::Chats)
}

/// Раздел «Статьи».
#[component]
pub fn AdminArticles() -> impl IntoView {
    page(Section::Articles)
}

/// Раздел «Кросспостинг».
#[component]
pub fn AdminOutbox() -> impl IntoView {
    page(Section::Outbox)
}

/// Раздел «Прайс».
#[component]
pub fn AdminPrices() -> impl IntoView {
    page(Section::Prices)
}

/// Раздел «Приём заявок».
#[component]
pub fn AdminSettings() -> impl IntoView {
    page(Section::Settings)
}

/// Оболочка админки: вход, сайдбар, шапка страницы и содержимое раздела.
fn page(section: Section) -> AnyView {
    view! {
        <section class="section admin-section">
            <div id="admin-app" class="admin-app">
                // Вход. Виден, пока в браузере нет токена: остальные ручки
                // ответят 401, и показывать пустые таблицы нечестно.
                <div id="admin-login" class="admin-login">
                    <h1 class="admin-login-title">"Админ-панель"</h1>
                    <p class="admin-page-sub">"Вход по токену — он же ключ к API админки."</p>
                    <label for="admin-token">"Токен доступа"</label>
                    <div class="admin-login-row">
                        <input
                            id="admin-token"
                            type="password"
                            autocomplete="off"
                            placeholder="Введите ADMIN_TOKEN"
                        />
                        <button id="admin-login-btn" type="button" class="btn btn-primary">
                            "Войти"
                        </button>
                    </div>
                    <p id="admin-login-status" class="form-status" role="status" aria-live="polite"></p>
                </div>

                // Оболочка раздела. `hidden` до входа: без токена данные не
                // придут, а полусобранная админка читается как поломка.
                <div id="admin-shell" class="admin-shell" hidden>
                    {sidebar(section)}

                    <div class="admin-main">
                        <header class="admin-page-head">
                            <div class="admin-page-heading">
                                <h1>{section.title()}</h1>
                                <p class="admin-page-sub">{section.subtitle()}</p>
                            </div>
                            {action(section)}
                        </header>

                        {content(section)}
                    </div>
                </div>
            </div>
        </section>
    }
    .into_any()
}

/// Сайдбар: разделы, сгруппированные по смыслу.
///
/// Группы — не украшение: в плоском списке из шести пунктов «Заявки» и
/// «Статьи» равноудалены, хотя это разные работы (клиенты и контент). Порядок
/// внутри группы — по частоте: к контенту заходят реже, чем к заявкам.
fn sidebar(current: Section) -> AnyView {
    view! {
        <aside class="admin-sidebar">
            <a class="admin-brand" href="/admin">
                <span class="admin-brand-mark" aria-hidden="true">"▚"</span>
                "Админка"
            </a>

            <nav class="admin-nav" aria-label="Разделы админки">
                {nav_group(None, &[Section::Overview], current)}
                {nav_group(Some("Клиенты"), &[Section::Leads, Section::Chats], current)}
                {nav_group(Some("Контент"), &[Section::Articles, Section::Outbox], current)}
                {nav_group(Some("Сайт"), &[Section::Prices, Section::Settings], current)}
            </nav>

            <a class="admin-nav-link admin-nav-exit" href="/">
                <span aria-hidden="true">"←"</span>
                "На сайт"
            </a>
        </aside>
    }
    .into_any()
}

/// Группа пунктов меню: подпись (если есть) и ссылки.
fn nav_group(title: Option<&'static str>, items: &[Section], current: Section) -> AnyView {
    view! {
        <div class="admin-nav-group">
            {title.map(|t| view! { <p class="admin-nav-title">{t}</p> })}
            {items
                .iter()
                .map(|&item| nav_link(item, current))
                .collect::<Vec<_>>()}
        </div>
    }
    .into_any()
}

/// Пункт меню раздела. Текущий помечен и классом, и `aria-current`.
fn nav_link(section: Section, current: Section) -> AnyView {
    let active = section == current;
    view! {
        <a
            class=if active { "admin-nav-link is-active" } else { "admin-nav-link" }
            href=section.path()
            aria-current=active.then_some("page")
        >
            {section.title()}
        </a>
    }
    .into_any()
}

/// Действие раздела в шапке страницы.
fn action(section: Section) -> AnyView {
    match section {
        // Единственный раздел с основным действием: все остальные только
        // читают данные или правят их на месте (статус заявки, настройки).
        Section::Articles => view! {
            <button type="button" id="article-new-btn" class="btn btn-primary">
                "Новая статья"
            </button>
        }
        .into_any(),
        _ => ().into_any(),
    }
}

/// Содержимое раздела.
fn content(section: Section) -> AnyView {
    match section {
        Section::Overview => overview(),
        Section::Leads => leads(),
        Section::Chats => chats(),
        Section::Articles => articles(),
        Section::Outbox => outbox(),
        Section::Prices => prices(),
        Section::Settings => settings(),
    }
}

/// Обзор: числа дня, разбивки и переходы в разделы.
fn overview() -> AnyView {
    view! {
        <div class="kpi-cards" id="admin-kpi"></div>

        <div class="admin-panels">
            <section class="admin-panel">
                <h2>"Источники заявок"</h2>
                <p class="admin-hint">"Откуда пришли заявки за всё время."</p>
                <div class="admin-bars" id="admin-lead-sources"></div>
            </section>
            <section class="admin-panel">
                <h2>"Типы вопросов"</h2>
                <p class="admin-hint">"Что чаще всего спрашивают в чате."</p>
                <div class="admin-bars" id="admin-question-types"></div>
            </section>
        </div>

        <nav class="admin-quick" aria-label="Разделы админки">
            {[
                (Section::Leads, "Заявки с контактами и статусами"),
                (Section::Chats, "История вопросов и ответов"),
                (Section::Articles, "Черновики, публикации и редактор"),
                (Section::Settings, "Статус приёма заявок в шапке"),
            ]
            .into_iter()
            .map(|(target, hint)| view! {
                <a class="admin-quick-link" href=target.path()>
                    <span class="admin-quick-title">
                        {target.title()}
                        <span aria-hidden="true">"→"</span>
                    </span>
                    <span class="admin-quick-hint">{hint}</span>
                </a>
            })
            .collect::<Vec<_>>()}
        </nav>
    }
    .into_any()
}

/// Прайс: редактируемая таблица строк.
///
/// Разметку строк собирает JS (как и остальные таблицы админки): колонки
/// правятся на месте, и дублировать их в шаблоне на Rust незачем. Сервер
/// задаёт контейнер, кнопки и пустое состояние.
fn prices() -> AnyView {
    view! {
        <p class="admin-hint">
            "Цены читает ассистент в чате и на сайте. Пустая строка «до» — открытая вилка, \
             пустые обе — «по запросу»."
        </p>
        <div class="admin-toolbar admin-prices-toolbar">
            <button type="button" id="price-add-btn" class="btn btn-secondary">
                "Добавить строку"
            </button>
            <button type="button" id="price-save-btn" class="btn btn-primary">
                "Сохранить прайс"
            </button>
            <span id="price-status" class="form-status" role="status" aria-live="polite"></span>
        </div>
        <div class="table-wrap">
            <table id="admin-prices-table" class="admin-table admin-prices" aria-label="Прайс-лист"></table>
        </div>
    }
    .into_any()
}

/// Заявки: полная таблица со сменой статуса.
fn leads() -> AnyView {
    view! {
        <div class="table-wrap">
            <table id="admin-leads-table" class="admin-table" aria-label="Заявки"></table>
        </div>
    }
    .into_any()
}

/// Чаты: история вопросов и ответов.
fn chats() -> AnyView {
    view! {
        <div class="table-wrap">
            <table id="admin-chats-table" class="admin-table" aria-label="История чатов"></table>
        </div>
    }
    .into_any()
}

/// Статьи: фильтр по статусу, таблица и редактор.
fn articles() -> AnyView {
    view! {
        <div class="admin-toolbar">
            <div
                class="admin-filters"
                id="admin-article-filters"
                role="group"
                aria-label="Фильтр по статусу"
            >
                <button type="button" class="filter-btn is-active" data-status="">"Все"</button>
                <button type="button" class="filter-btn" data-status="draft">"Черновики"</button>
                <button type="button" class="filter-btn" data-status="published">"Опубликованные"</button>
                <button type="button" class="filter-btn" data-status="archived">"Архив"</button>
            </div>
        </div>
        <p class="admin-hint" id="admin-articles-summary"></p>

        <div class="table-wrap">
            <table id="admin-articles-table" class="admin-table" aria-label="Статьи"></table>
        </div>

        // Редактор: скрыт, пока не выбрана статья или не нажата «Новая статья».
        <div id="article-editor" class="admin-editor" hidden>
            <h3 id="article-editor-heading">"Новая статья"</h3>

            <div class="admin-editor-grid">
                <div class="admin-editor-form">
                    <label for="article-title">"Заголовок"</label>
                    <input id="article-title" type="text" maxlength="200"/>

                    <label for="article-slug">"Адрес (slug)"</label>
                    <input
                        id="article-slug"
                        type="text"
                        maxlength="120"
                        placeholder="пусто — сгенерируется из заголовка"
                    />
                    <p class="field-hint">
                        "Латиница, цифры и дефисы. Меняйте только осознанно: адрес попадает в ссылки и RSS."
                    </p>

                    <label for="article-summary">"Краткое описание"</label>
                    <textarea id="article-summary" rows="2" maxlength="500"></textarea>
                    <p class="field-hint">
                        "Показывается в списке блога, RSS и в описании для поисковиков."
                    </p>

                    <label for="article-tags">"Теги"</label>
                    <input id="article-tags" type="text" placeholder="rust, ai, mcp"/>
                    <p class="field-hint">"Через запятую, не больше 10."</p>

                    <label for="article-cover">"Обложка"</label>
                    <div id="article-cover-dropzone" class="dropzone dropzone-slim">
                        <input id="article-cover" type="text" placeholder="https://… или /media/…"/>
                        <div class="dropzone-overlay" aria-hidden="true">
                            <span>"Отпустите, чтобы сделать обложкой"</span>
                        </div>
                    </div>
                    <p class="field-hint">
                        "Перетащите изображение сюда или вставьте из буфера — оно загрузится, а адрес подставится сам."
                    </p>
                    <div id="article-cover-preview" class="cover-preview" hidden>
                        <img id="article-cover-preview-img" alt="" loading="lazy"/>
                        <button type="button" id="article-cover-clear" class="mini-btn">"Убрать"</button>
                    </div>

                    <label for="article-canonical">"Канонический адрес"</label>
                    <input
                        id="article-canonical"
                        type="text"
                        placeholder="если статья уже опубликована в другом месте"
                    />

                    <label for="article-habr">"Читать на Хабр"</label>
                    <input id="article-habr" type="text" placeholder="https://habr.com/ru/articles/…"/>
                    <p class="field-hint">
                        "Полная ссылка на внешнюю публикацию этой статьи. Пусто — блок не показывается. В отличие от канонического адреса это ссылка для читателя, и она появляется в начале текста."
                    </p>

                    <label for="article-status">"Статус"</label>
                    <select id="article-status">
                        <option value="draft">"Черновик — не виден на сайте"</option>
                        <option value="published">"Опубликована — видна в блоге"</option>
                        <option value="archived">"В архиве — снята с публикации"</option>
                    </select>

                    <label for="article-body">"Текст (Markdown)"</label>
                    <div id="article-body-dropzone" class="dropzone">
                        <textarea
                            id="article-body"
                            class="admin-editor-body"
                            rows="18"
                            placeholder="# Заголовок&#10;&#10;Текст статьи…"
                        ></textarea>
                        <div class="dropzone-overlay" aria-hidden="true">
                            <span>"Отпустите, чтобы вставить изображение в текст"</span>
                        </div>
                    </div>
                    <p class="field-hint">
                        "Перетащите картинку в поле или вставьте из буфера (Ctrl+V) — она загрузится и встанет в текст на месте курсора."
                    </p>
                    <p id="article-media-status" class="form-status" role="status" aria-live="polite"></p>

                    <div class="admin-editor-actions">
                        <button type="button" id="article-save-btn" class="btn btn-primary">"Сохранить"</button>
                        <button type="button" id="article-preview-btn" class="btn btn-secondary">
                            "Предпросмотр"
                        </button>
                        <button type="button" id="article-image-btn" class="btn btn-secondary">
                            "Вставить изображение"
                        </button>
                        <button type="button" id="article-cancel-btn" class="btn btn-secondary">"Закрыть"</button>
                        <input
                            id="article-image-input"
                            type="file"
                            hidden
                            accept="image/png,image/jpeg,image/webp,image/gif,image/avif"
                            multiple
                        />
                    </div>
                    <p id="article-editor-status" class="form-status" role="status" aria-live="polite"></p>
                </div>

                <div class="admin-editor-preview">
                    <div class="admin-editor-preview-head">
                        <strong>"Предпросмотр"</strong>
                        <span id="article-preview-meta" class="admin-hint"></span>
                    </div>
                    <div id="article-preview" class="prose admin-preview-body">
                        <p class="admin-hint">
                            "Нажмите «Предпросмотр», чтобы увидеть, как статья будет выглядеть на сайте."
                        </p>
                    </div>
                </div>
            </div>
        </div>
    }
    .into_any()
}

/// Кросспостинг: очередь событий для n8n.
fn outbox() -> AnyView {
    view! {
        <p class="admin-hint">
            "Публикация статьи создаёт событие в той же транзакции, что и запись статьи: очередь не разъезжается с блогом."
        </p>
        <p class="admin-hint" id="admin-outbox-summary"></p>
        <div class="table-wrap">
            <table id="admin-outbox-table" class="admin-table" aria-label="События кросспостинга"></table>
        </div>
    }
    .into_any()
}

/// Приём заявок: бейдж в шапке сайта.
///
/// Значения те же, что у Telegram-бота (`services::settings_bot`): границы
/// полей берутся из его констант, а не пишутся здесь заново.
fn settings() -> AnyView {
    view! {
        <div class="admin-settings" id="admin-settings">
            <div class="admin-settings-grid">
                <div class="form-field">
                    <label for="admin-status-select">"Статус"</label>
                    <select id="admin-status-select">
                        <option value="available">"Открыт — беру новые проекты"</option>
                        <option value="busy">"Ограничен — беру не все"</option>
                        <option value="full">"Закрыт — новые не беру"</option>
                    </select>
                </div>
                <div class="form-field">
                    <label for="admin-projects">"Проектов в работе"</label>
                    <input
                        id="admin-projects"
                        type="number"
                        min="0"
                        max=MAX_PROJECTS.to_string()
                        step="1"
                        inputmode="numeric"
                    />
                </div>
                <div class="form-field">
                    <label for="admin-slot">"Ближайший слот"</label>
                    <input
                        id="admin-slot"
                        type="text"
                        maxlength=TEXT_FIELD_MAX_CHARS.to_string()
                        placeholder="например: с 1 октября"
                    />
                </div>
            </div>
            <div class="admin-editor-actions">
                <button type="button" id="admin-settings-save" class="btn btn-primary">"Сохранить"</button>
            </div>
            <p id="admin-settings-message" class="form-status" role="status" aria-live="polite"></p>
        </div>
        <p class="admin-hint">
            "Правки видны сразу на всех страницах сайта: бейдж в шапке читает то же состояние, что и Telegram-бот."
        </p>
    }
    .into_any()
}

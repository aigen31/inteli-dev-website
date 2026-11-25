//! Лендинг CRM: швейцарская сетка, один акцент, продуктовые мокапы и Кики.

use leptos::prelude::*;
use leptos::svg;

const CRM_URL: &str = "https://crm.inteli-dev.ru";

/// Точная разметка кинетической сцены из лаборатории.
const CRM_LAB_SCENE: &str = include_str!("../../assets/crm-lab-scene.html");

/// Линейный глиф для шагов и карточек: визуальный ориентир без чтения.
#[component]
fn CrmGlyph(#[prop(into)] kind: String) -> impl IntoView {
    let paths = match kind.as_str() {
        "client" => r#"<circle cx="12" cy="8" r="3"/><path d="M5 20c0-3.6 3.1-6 7-6s7 2.4 7 6"/>"#,
        "project" => r#"<path d="M3 6h6l2 2h10v11H3z"/><path d="M3 10h18"/>"#,
        "task" => r#"<rect x="4" y="4" width="16" height="16" rx="2"/><path d="m8 12 3 3 5-6"/>"#,
        "invoice" => r#"<path d="M6 3h9l4 4v14H6z"/><path d="M15 3v4h4"/><path d="M9 12h6M9 16h4"/>"#,
        "balance" => r#"<rect x="3" y="6" width="18" height="13" rx="2"/><path d="M3 10h18M16 15h2"/>"#,
        "kiki" => r#"<path d="M12 3v4M12 17v4M3 12h4M17 12h4"/><path d="m6 6 2 2M16 16l2 2M18 6l-2 2M8 16l-2 2"/>"#,
        "privacy" => r#"<path d="M12 3 4 7v5c0 5 3.5 8 8 9 4.5-1 8-4 8-9V7z"/><path d="m9 12 2 2 4-4"/>"#,
        "load" => r#"<rect x="3" y="5" width="18" height="16" rx="2"/><path d="M8 3v4M16 3v4M3 10h18M8 14h2M14 14h2M8 18h2"/>"#,
        "context" => r#"<path d="m12 3 9 5-9 5-9-5z"/><path d="m3 13 9 5 9-5"/>"#,
        _ => r#"<circle cx="12" cy="12" r="8"/>"#,
    };
    svg::svg()
        .attr("xmlns", "http://www.w3.org/2000/svg")
        .attr("viewBox", "0 0 24 24")
        .attr("fill", "none")
        .attr("stroke", "currentColor")
        .attr("stroke-width", "1.25")
        .attr("stroke-linecap", "round")
        .attr("stroke-linejoin", "round")
        .attr("aria-hidden", "true")
        .attr("class", "crm-glyph")
        .inner_html(paths)
}

/// Короткая секция на главной: что это и куда идти.
#[component]
pub fn CrmTeaser() -> impl IntoView {
    view! {
        <section class="section section-crm-teaser" id="crm">
            <div class="section-head">
                <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>"03 / CRM"</p>
                <h2>"CRM, в которой я веду клиентов"</h2>
                <a class="section-link" href="/crm">"Открыть лендинг"</a>
            </div>

            <div class="crm-teaser-grid" data-crm-reveal>
                <div class="crm-teaser-copy">
                    <p class="crm-teaser-lead">
                        "Клиенты, проекты, задачи, счета и баланс в одном месте. "
                        "Завершаю задачу, списываю оплату с баланса, выставляю счёт. "
                        "Без таблиц и переключения между сервисами."
                    </p>
                    <ul class="crm-teaser-points">
                        <li><span class="mono">01</span>"Карточка клиента: проекты, задачи, счета, история"</li>
                        <li><span class="mono">02</span>"Задачи и баланс связаны: счёт рождается из задачи"</li>
                        <li><span class="mono">03</span>"Кики находит задачи и готовит счёт по команде"</li>
                    </ul>
                    <a class="btn btn-primary crm-teaser-cta" href="/crm">
                        "Как работает CRM"
                        <span aria-hidden="true">"↗"</span>
                    </a>
                </div>

                <div class="crm-teaser-art" aria-hidden="true">
                    <div class="crm-teaser-halo"></div>
                    <img src="/assets/crm/kiki-crm-desk.webp?v=4" alt="" width="1440" height="1440" loading="lazy" decoding="async"/>
                    <div class="crm-teaser-chip crm-teaser-chip-a mono">"tasks / 12"</div>
                    <div class="crm-teaser-chip crm-teaser-chip-b mono">"balance / ok"</div>
                </div>
            </div>

            <div class="crm-teaser-stats" data-crm-reveal>
                <div><span class="mono">"01 / Клиенты"</span><p>"карточка, история, проекты"</p></div>
                <div><span class="mono">"02 / Работа"</span><p>"задачи, сроки, отчёты"</p></div>
                <div><span class="mono">"03 / Деньги"</span><p>"счёт, оплата, баланс"</p></div>
                <a class="crm-teaser-arrow" href="/crm" aria-label="Перейти на лендинг CRM">"→"</a>
            </div>
        </section>
    }
}

#[component]
pub fn CrmLanding() -> impl IntoView {
    view! {
        <section class="crm-page">
            <div class="crm-noise" aria-hidden="true"></div>

            <div class="crm-hero crm-hero-full" data-crm-hero>
                <div class="crm-hero-top mono">
                    <span>"INTELI DEV / CRM"</span>
                    <span>"2026"</span>
                    <span>"КРАСНОЯРСК / УДАЛЁННО"</span>
                </div>

                <div class="crm-hero-copy">
                    <h1 class="crm-title">
                        <span class="crm-title-line" data-crm-reveal>"Клиенты. Задачи."</span>
                        <span class="crm-title-line" data-crm-reveal>"Счета. "<em class="crm-title-accent">"В одной CRM."</em></span>
                    </h1>

                    <div class="crm-hero-foot">
                        <p class="crm-lead" data-crm-reveal>
                            "CRM, в которой клиент, задача и деньги живут в одной истории. "
                            "Завершаю задачу, списываю оплату, выставляю счёт. "
                            "Без таблиц и переключения между вкладками."
                        </p>
                        <div class="crm-hero-actions" data-crm-reveal>
                            <a class="btn btn-primary" href=CRM_URL>
                                "Открыть CRM"
                                <span aria-hidden="true">"↗"</span>
                            </a>
                            <a class="crm-text-link" href="#crm-flow">"Как это работает" <span aria-hidden="true">"↓"</span></a>
                        </div>
                        <div class="crm-hero-facts mono" aria-label="Что внутри CRM">
                            <span><b>"01"</b>"Клиенты"</span>
                            <span><b>"02"</b>"Задачи"</span>
                            <span><b>"03"</b>"Счета"</span>
                            <span><b>"04"</b>"Баланс"</span>
                        </div>
                    </div>
                </div>

                <div class="crm-hero-lab">
                    <div class="crm-lab" inner_html=CRM_LAB_SCENE></div>
                </div>

            </div>

            <section class="crm-manifesto">
                <div class="crm-section-label mono" data-crm-reveal>"01 / Принцип"</div>
                <h2 class="crm-h2" data-crm-reveal>"Клиент, работа и деньги в одной базе."</h2>
                <div class="crm-manifesto-grid">
                    <article data-crm-reveal>
                        <span class="crm-num mono">"A"</span>
                        <span class="crm-article-glyph"><CrmGlyph kind="client"/></span>
                        <h3>"Клиент"</h3>
                        <p>"Карточка, история, проекты, счета и переписка. Всё, что было раньше, под рукой."</p>
                    </article>
                    <article data-crm-reveal>
                        <span class="crm-num mono">"B"</span>
                        <span class="crm-article-glyph"><CrmGlyph kind="task"/></span>
                        <h3>"Работа"</h3>
                        <p>"Проект, задача, срок, исполнитель, отчёт. Завершённая задача меняет баланс клиента."</p>
                    </article>
                    <article data-crm-reveal>
                        <span class="crm-num mono">"C"</span>
                        <span class="crm-article-glyph"><CrmGlyph kind="balance"/></span>
                        <h3>"Деньги"</h3>
                        <p>"Счёт, оплата и баланс в одной истории. Минус видно сразу, а не после сверки."</p>
                    </article>
                </div>
            </section>

            <section class="crm-flow" id="crm-flow" data-crm-flow>
                <div class="crm-flow-head">
                    <div>
                        <div class="crm-section-label mono" data-crm-reveal>"02 / Поток"</div>
                        <h2 class="crm-h2" data-crm-reveal>"Пять шагов от заявки до оплаты."</h2>
                    </div>
                    <p class="crm-flow-note" data-crm-reveal>
                        "У каждого шага свой экран и свои данные. Следующий шаг уже знает, что произошло на предыдущем."
                    </p>
                </div>

                <div class="crm-flow-track">
                    <div class="crm-flow-rail" aria-hidden="true"><span class="crm-flow-rail-fill"></span></div>
                    <ol class="crm-flow-steps">
                        <li data-crm-reveal>
                            <div class="crm-step-head">
                                <span class="crm-step-glyph"><CrmGlyph kind="client"/></span>
                                <span class="crm-step-index mono">"01"</span>
                            </div>
                            <h3>"Клиент"</h3>
                            <p>"Заявка приходит из формы, брифа или разговора. Контакты, согласие и история остаются в карточке."</p>
                            <span class="crm-step-tag mono">"crm.clients"</span>
                        </li>
                        <li data-crm-reveal>
                            <div class="crm-step-head">
                                <span class="crm-step-glyph"><CrmGlyph kind="project"/></span>
                                <span class="crm-step-index mono">"02"</span>
                            </div>
                            <h3>"Проект"</h3>
                            <p>"Ключ, участники, доступы, сайт и AGENTS.md. Контекст не теряется между задачами."</p>
                            <span class="crm-step-tag mono">"crm.projects"</span>
                        </li>
                        <li data-crm-reveal>
                            <div class="crm-step-head">
                                <span class="crm-step-glyph"><CrmGlyph kind="task"/></span>
                                <span class="crm-step-index mono">"03"</span>
                            </div>
                            <h3>"Задача"</h3>
                            <p>"Исполнитель видит свои задачи, клиент свои. Статусы, чек-листы, отчёты и календарь нагрузки."</p>
                            <span class="crm-step-tag mono">"crm.tasks"</span>
                        </li>
                        <li data-crm-reveal>
                            <div class="crm-step-head">
                                <span class="crm-step-glyph"><CrmGlyph kind="invoice"/></span>
                                <span class="crm-step-index mono">"04"</span>
                            </div>
                            <h3>"Счёт"</h3>
                            <p>"Счёт рождается из задачи. Перевод или ЮKassa, подтверждение, оплата в одном журнале."</p>
                            <span class="crm-step-tag mono">"crm.invoices"</span>
                        </li>
                        <li data-crm-reveal>
                            <div class="crm-step-head">
                                <span class="crm-step-glyph"><CrmGlyph kind="balance"/></span>
                                <span class="crm-step-index mono">"05"</span>
                            </div>
                            <h3>"Баланс"</h3>
                            <p>"Завершённая задача списывает сумму с баланса. В истории операций указана задача."</p>
                            <span class="crm-step-tag mono">"crm.balance"</span>
                        </li>
                    </ol>
                </div>
            </section>

            <section class="crm-surface">
                <div class="crm-section-label mono" data-crm-reveal>"03 / Для кого"</div>
                <h2 class="crm-h2" data-crm-reveal>"CRM для одного человека или небольшой команды."</h2>
                <p class="crm-surface-note" data-crm-reveal>
                    "Большие CRM строят вокруг отдела продаж: воронки, роли, долгое внедрение. "
                    "Здесь работают владелец и несколько исполнителей. Клиенты, задачи и деньги без корпоративной обвязки."
                </p>

                <div class="crm-compare" data-crm-reveal>
                    <div class="crm-compare-col">
                        <span class="mono">"Большая CRM"</span>
                        <ul>
                            <li>"Воронка и отдел продаж"</li>
                            <li>"Оплата за каждого пользователя"</li>
                            <li>"Модули, которые не нужны"</li>
                        </ul>
                    </div>
                    <div class="crm-compare-col is-ours">
                        <span class="mono">"Эта CRM"</span>
                        <ul>
                            <li>"Владелец и небольшая команда"</li>
                            <li>"Оплата не зависит от числа клиентов"</li>
                            <li>"Клиенты, задачи, счета и баланс"</li>
                        </ul>
                    </div>
                </div>

                <div class="crm-bento" data-crm-reveal>
                    <article class="crm-bento-card crm-bento-board" data-crm-tilt>
                        <header><span class="mono">"tasks"</span><span class="crm-dot"></span></header>
                        <h3>"Что в работе"</h3>
                        <div class="crm-task-row"><span class="crm-task-code mono">"KOR-18"</span><span>"Лендинг для кофейни"</span><em>"в работе"</em></div>
                        <div class="crm-task-row"><span class="crm-task-code mono">"KOR-21"</span><span>"Календарь нагрузки"</span><em>"на проверке"</em></div>
                        <div class="crm-task-row crm-task-muted"><span class="crm-task-code mono">"KOR-22"</span><span>"Счёт за поддержку"</span><em>"очередь"</em></div>
                    </article>

                    <article class="crm-bento-card crm-bento-calendar" data-crm-tilt>
                        <header><span class="mono">"load"</span><span class="mono">"week 47"</span></header>
                        <h3>"Нагрузка"</h3>
                        <div class="crm-calendar-strip" aria-hidden="true">
                            <span class="is-busy">"24"</span><span class="is-busy">"25"</span><span class="is-busy">"26"</span>
                            <span class="is-busy">"27"</span><span class="is-free">"28"</span><span class="is-free">"29"</span><span class="is-free">"30"</span>
                        </div>
                        <p class="crm-mono-note mono">"5+ задач: день занят"</p>
                    </article>

                    <article class="crm-bento-card crm-bento-money" data-crm-tilt>
                        <header><span class="mono">"balance"</span><span class="mono">"RUB"</span></header>
                        <h3>"−1 500,00 ₽"</h3>
                        <p class="crm-mono-note mono">"списание за задачу «Лендинг»"</p>
                        <div class="crm-balance-line"><span></span></div>
                    </article>

                    <article class="crm-bento-card crm-bento-client" data-crm-tilt>
                        <header><span class="mono">"client"</span><span class="mono">"ACTIVE"</span></header>
                        <h3>"Один клиент"</h3>
                        <p class="crm-mono-note mono">"3 проекта · 12 задач · 4 счёта"</p>
                        <div class="crm-client-lines" aria-hidden="true"><span></span><span></span><span></span></div>
                    </article>
                </div>
            </section>

            <section class="crm-kiki">
                <div class="crm-kiki-art" data-crm-reveal>
                    <img src="/assets/crm/kiki-crm-grid.webp?v=4" alt="Кики сидит в воздухе" width="700" height="1084" loading="lazy" decoding="async"/>
                </div>
                <div class="crm-kiki-copy">
                    <div class="crm-section-label mono" data-crm-reveal>"04 / Кики"</div>
                    <h2 class="crm-h2" data-crm-reveal>"Кики выполняет действия по команде."</h2>
                    <p data-crm-reveal>
                        "Она вызывает инструменты CRM: находит задачи на дату, готовит счёт, меняет статус, читает документ. "
                        "Вы видите, какой инструмент сработал и с какими данными."
                    </p>
                    <div class="crm-tool-chips" data-crm-reveal>
                        <span class="mono">"list_tasks"</span>
                        <span class="mono">"create_invoice"</span>
                        <span class="mono">"get_balance"</span>
                        <span class="mono">"read_document"</span>
                    </div>
                    <div class="crm-chat-mock is-thinking" data-crm-reveal data-crm-chat aria-hidden="true">
                        <div class="crm-chat-line crm-chat-user">"Покажи задачи на завтра"</div>
                        <div class="crm-chat-line crm-chat-kiki">
                            <span class="crm-typing"><i></i><i></i><i></i></span>
                            <span class="crm-chat-tool mono">"list_tasks · date=2026-11-25"</span>
                        </div>
                        <div class="crm-chat-line crm-chat-result">
                            <span class="crm-chat-ok">"✓"</span>
                            <span>"3 задачи · 1 на проверке"</span>
                        </div>
                        <div class="crm-chat-tasks">
                            <span>"Лендинг"</span><span>"Календарь"</span><span>"Счёт"</span>
                        </div>
                        <div class="crm-chat-status mono" data-crm-chat-status>"Кики смотрит задачи"</div>
                    </div>
                </div>
            </section>

            <section class="crm-features">
                <div class="crm-section-label mono" data-crm-reveal>"05 / Отличия"</div>
                <h2 class="crm-h2" data-crm-reveal>"Что здесь устроено иначе."</h2>
                <div class="crm-feature-grid">
                    <article data-crm-reveal>
                        <div class="crm-feature-top"><CrmGlyph kind="balance"/><span class="mono">"01"</span></div>
                        <h3>"Задача меняет баланс"</h3>
                        <p>"Закрыл задачу, сумма списалась с баланса клиента. В истории операций видна сама задача."</p>
                        <span class="crm-feature-tag mono">"задача → списание"</span>
                    </article>
                    <article data-crm-reveal>
                        <div class="crm-feature-top"><CrmGlyph kind="invoice"/><span class="mono">"02"</span></div>
                        <h3>"Счёт рождается из задачи"</h3>
                        <p>"Строки и сумма берутся из задачи. Не нужно собирать счёт вручную и сверять с таблицей."</p>
                        <span class="crm-feature-tag mono">"задача → счёт"</span>
                    </article>
                    <article data-crm-reveal>
                        <div class="crm-feature-top"><CrmGlyph kind="kiki"/><span class="mono">"03"</span></div>
                        <h3>"Кики вызывает инструменты"</h3>
                        <p>"Находит задачи, готовит счёт, меняет статус. Видно, какой инструмент сработал и с какими данными."</p>
                        <span class="crm-feature-tag mono">"list_tasks · create_invoice"</span>
                    </article>
                    <article data-crm-reveal>
                        <div class="crm-feature-top"><CrmGlyph kind="privacy"/><span class="mono">"04"</span></div>
                        <h3>"Клиент видит только своё"</h3>
                        <p>"Свои задачи, счета и баланс. Чужие проекты, клиенты и переписка недоступны."</p>
                        <span class="crm-feature-tag mono">"роль → доступ"</span>
                    </article>
                    <article data-crm-reveal>
                        <div class="crm-feature-top"><CrmGlyph kind="load"/><span class="mono">"05"</span></div>
                        <h3>"Нагрузка по дням"</h3>
                        <p>"В календаре видно, сколько задач уже назначено. Пять и больше задач помечают день занятым."</p>
                        <span class="crm-feature-tag mono">"5+ задач = занято"</span>
                    </article>
                    <article data-crm-reveal>
                        <div class="crm-feature-top"><CrmGlyph kind="context"/><span class="mono">"06"</span></div>
                        <h3>"Проект несёт контекст"</h3>
                        <p>"Ключ, доступы, сайт, AGENTS.md и участники. Следующая задача начинается не с нуля."</p>
                        <span class="crm-feature-tag mono">"проект → задача"</span>
                    </article>
                </div>
            </section>

            <section class="crm-repos" id="crm-repos">
                <div class="crm-repos-head">
                    <div>
                        <p class="crm-section-label mono" data-crm-reveal>"06 / Репозитории"</p>
                        <h2 class="crm-h2" data-crm-reveal>"Код можно читать."</h2>
                    </div>
                    <p class="crm-repos-note" data-crm-reveal>
                        "Три публичных репозитория: CRM, ядро Кики и этот сайт. "
                        "Без закрытых зависимостей и маркетинговых обещаний."
                    </p>
                </div>

                <div class="crm-repo-grid">
                    <a class="crm-repo crm-repo-wide" href="https://github.com/aigen31/inteli-dev-crm" target="_blank" rel="noopener noreferrer" data-crm-reveal>
                        <span class="crm-repo-index mono">"01"</span>
                        <div class="crm-repo-body">
                            <span class="crm-repo-kicker mono">"PHP / LARAVEL · VUE · POSTGRES"</span>
                            <h3>"inteli-dev-crm"</h3>
                            <p>"Рабочее пространство: клиенты, брифы, проекты, задачи, счета, баланс и Кики. Роли, доступы и история операций в одном контуре."</p>
                            <span class="crm-repo-link mono">"github.com/aigen31/inteli-dev-crm" <span aria-hidden="true">"↗"</span></span>
                        </div>
                        <span class="crm-repo-mark" aria-hidden="true">"↗"</span>
                    </a>

                    <a class="crm-repo" href="https://github.com/aigen31/inteli-dev-kiki-assistant-kernel-api" target="_blank" rel="noopener noreferrer" data-crm-reveal>
                        <span class="crm-repo-index mono">"02"</span>
                        <div class="crm-repo-body">
                            <span class="crm-repo-kicker mono">"RUST · AXUM · SQLITE"</span>
                            <h3>"kiki-assistant-kernel-api"</h3>
                            <p>"Ядро ассистента: диалоги, инструменты, документы, изображения и маршрутизация моделей для сайта и CRM."</p>
                            <span class="crm-repo-link mono">"github.com/aigen31/…kernel-api" <span aria-hidden="true">"↗"</span></span>
                        </div>
                        <span class="crm-repo-mark" aria-hidden="true">"↗"</span>
                    </a>

                    <a class="crm-repo" href="https://github.com/aigen31/inteli-dev-website" target="_blank" rel="noopener noreferrer" data-crm-reveal>
                        <span class="crm-repo-index mono">"03"</span>
                        <div class="crm-repo-body">
                            <span class="crm-repo-kicker mono">"RUST · LEPTOS · AXUM"</span>
                            <h3>"inteli-dev-website"</h3>
                            <p>"Этот сайт: услуги, кейсы, блог, CRM-лендинг, чат с Кики и админка. SSR без Node.js на продакшене."</p>
                            <span class="crm-repo-link mono">"github.com/aigen31/inteli-dev-website" <span aria-hidden="true">"↗"</span></span>
                        </div>
                        <span class="crm-repo-mark" aria-hidden="true">"↗"</span>
                    </a>
                </div>
            </section>

            <section class="crm-final">
                <div class="crm-final-inner" data-crm-reveal>
                    <p class="crm-section-label mono">"07 / Доступ"</p>
                    <h2>"Показать CRM на ваших задачах."</h2>
                    <p>"Созвон: покажу клиентов, задачи, счета и баланс на примере вашего процесса. Если не подойдёт, скажу прямо."</p>
                    <div class="crm-hero-actions">
                        <a class="btn btn-primary" href=CRM_URL>"Открыть CRM" <span aria-hidden="true">"↗"</span></a>
                        <a class="crm-text-link" href="/contact#lead-form" data-crm-access>"Запросить доступ" <span aria-hidden="true">"→"</span></a>
                    </div>
                </div>
                <div class="crm-final-meta mono">
                    <span>"INTELI DEV"</span>
                    <span>"CRM / 2026"</span>
                    <span>"LEAN LARAVEL + VUE"</span>
                </div>
            </section>

            <div class="crm-modal" id="crm-access-modal" hidden>
                <div class="crm-modal-backdrop" data-crm-access-close></div>
                <div class="crm-modal-panel" role="dialog" aria-modal="true" aria-labelledby="crm-access-title" tabindex="-1">
                    <button type="button" class="crm-modal-close" data-crm-access-close aria-label="Закрыть форму">"×"</button>
                    <p class="crm-section-label mono">"Доступ к CRM"</p>
                    <h2 id="crm-access-title">"Запросить доступ"</h2>
                    <p class="crm-modal-lead">"Напишите, что хотите вести в CRM. Отвечу на почту и покажу демо."</p>
                    <form class="crm-modal-form" id="crm-access-form">
                        <label class="crm-modal-field">
                            <span>"Имя"</span>
                            <input type="text" name="name" autocomplete="name" required maxlength="120"/>
                        </label>
                        <label class="crm-modal-field">
                            <span>"Email"</span>
                            <input type="email" name="email" autocomplete="email" required maxlength="160"/>
                        </label>
                        <label class="crm-modal-field">
                            <span>"Что хотите вести"</span>
                            <textarea name="message" rows="3" required maxlength="2000" placeholder="Клиенты, проекты, задачи, счета"></textarea>
                        </label>
                        <label class="crm-modal-consent">
                            <input type="checkbox" name="consent" required/>
                            <span>"Согласен на " <a href="https://crm.inteli-dev.ru/legal/consent" target="_blank" rel="noopener noreferrer">"обработку персональных данных"</a></span>
                        </label>
                        <button type="submit" class="btn btn-primary">"Отправить запрос" <span aria-hidden="true">"↗"</span></button>
                        <p class="crm-modal-status" id="crm-access-status" role="status"></p>
                    </form>
                </div>
            </div>
        </section>
    }
}

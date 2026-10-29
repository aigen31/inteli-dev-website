//! Общие UI-компоненты: кнопки, бейдж статуса, карточки.

use leptos::prelude::*;

use crate::memory::content::{Project, Service};
use crate::services::status;
use crate::settings::SiteSettings;
use crate::ui::icon::LucideIcon;

/// Бейдж текущего статуса занятости.
///
/// Читает [`SiteSettings`], а не контент: статус правится из Telegram-бота, и
/// бейдж в шапке должен меняться без перезапуска.
///
/// Форма: постоянное подлежащее + короткое состояние («Приём заявок: открыт»).
/// Подлежащее не меняется от состояния к состоянию — иначе бейдж читается как
/// три разных текста, а «ограниченная доступность» без подлежащего непонятна
/// посетителю, который только зашёл. Цвет точки — дублирующий сигнал, не
/// единственный: словесное состояние видно и без наведения, и на телефоне, и
/// скринридеру (через `aria-label`).
///
/// Подсказка с подробностями — **дополнение**: раскрывается наведением (CSS),
/// фокусом с клавиатуры (CSS) и тапом (`initStatusBadge` в `assets/main.js`).
#[component]
pub fn StatusBadge() -> impl IntoView {
    let availability = SiteSettings::availability();
    let p = status::presentation(&availability.status);
    let lines = status::facts(&availability);
    let aria = status::aria_label(&availability);

    view! {
        <button
            type="button"
            class=format!("status-badge status-{}", availability.status)
            aria-expanded="false"
            aria-label=aria
        >
            <span class="status-dot" aria-hidden="true"></span>
            <span class="status-subject">{format!("{}:", p.subject)}</span>
            <span class="status-state">{p.state}</span>
            // `aria-hidden`: тот же текст уже в `aria-label`, дублировать его
            // для скринридера не нужно.
            <span class="status-popover" aria-hidden="true">
                <span class="status-popover-summary">{p.summary}</span>
                {lines
                    .into_iter()
                    .map(|line| view! { <span class="status-popover-line">{line}</span> })
                    .collect::<Vec<_>>()}
            </span>
        </button>
    }
}

/// Просмотр изображения статьи в полном размере.
///
/// Картинка в тексте ограничена шириной колонки: схему или скриншот в ней не
/// разглядеть. Здесь только разметка панели — показывает её клик по картинке
/// (`initLightbox` в `assets/main.js`), поэтому до клика панель выключена
/// атрибутом `hidden` и не читается ни скринридером, ни поиском.
///
/// Живёт в [`crate::ui::layout::Layout`], а не внутри статьи: `position: fixed`
/// ломается у предка с `opacity < 1` или `transform` — он становится содержащим
/// блоком. Секции на сайте проявляются именно так (`.section` в
/// `assets/style.css`), и панель на время анимации растянулась бы по статье
/// длиной в несколько экранов вместо окна браузера.
#[component]
pub fn Lightbox() -> impl IntoView {
    view! {
        <div
            class="lightbox"
            id="lightbox"
            hidden
            role="dialog"
            aria-modal="true"
            aria-label="Просмотр изображения"
        >
            <button
                type="button"
                class="lightbox-close"
                id="lightbox-close"
                aria-label="Закрыть просмотр"
            >
                <span aria-hidden="true">{"×"}</span>
            </button>

            // Стрелки скрыты до первого показа: галерея из одной картинки их не
            // показывает, а решает это уже скрипт.
            <button
                type="button"
                class="lightbox-nav lightbox-prev"
                id="lightbox-prev"
                aria-label="Предыдущее изображение"
                hidden
            >
                <span aria-hidden="true">{"‹"}</span>
            </button>

            <div class="lightbox-body">
                <figure class="lightbox-figure">
                    <img class="lightbox-img" id="lightbox-img" alt=""/>
                    <figcaption class="lightbox-caption" id="lightbox-caption" hidden></figcaption>
                </figure>
                <p class="lightbox-counter" id="lightbox-counter" hidden></p>
            </div>

            <button
                type="button"
                class="lightbox-nav lightbox-next"
                id="lightbox-next"
                aria-label="Следующее изображение"
                hidden
            >
                <span aria-hidden="true">{"›"}</span>
            </button>
        </div>
    }
}

/// Primary CTA-кнопка (ссылка).
#[component]
pub fn PrimaryButton(#[prop(into)] text: String, #[prop(into)] href: String) -> impl IntoView {
    view! { <a href=href class="btn btn-primary">{text}</a> }
}

/// Secondary CTA-кнопка (ссылка).
#[component]
pub fn SecondaryButton(#[prop(into)] text: String, #[prop(into)] href: String) -> impl IntoView {
    view! { <a href=href class="btn btn-secondary">{text}</a> }
}

/// Заголовок карточки нужного уровня.
///
/// Уровень — не украшение: краулер нашёл на `/services` и `/projects` пропуск
/// уровня (`<h1>` → `<h3>`) — карточки лежат там прямо в секции страницы. На
/// главной те же карточки стоят внутри секции с `<h2>`, и правильный уровень
/// для них — `<h3>`. Внешний вид задаёт класс, а не тег, поэтому уровень можно
/// менять, не трогая вёрстку и шрифт (`RULES/ui-rules.md` → «Иерархия
/// заголовков»).
///
/// `link` — адрес, на который ведёт заголовок (карточки статей), у карточек
/// услуг и кейсов заголовок не ссылка.
pub fn card_heading(level: u8, class: &'static str, title: String, link: Option<String>) -> AnyView {
    match (level, link) {
        (2, Some(href)) => view! { <h2 class=class><a href=href>{title}</a></h2> }.into_any(),
        (2, None) => view! { <h2 class=class>{title}</h2> }.into_any(),
        (_, Some(href)) => view! { <h3 class=class><a href=href>{title}</a></h3> }.into_any(),
        (_, None) => view! { <h3 class=class>{title}</h3> }.into_any(),
    }
}

/// Карточка услуги.
///
/// `level` — уровень заголовка: 2 на `/services` (карточка — самостоятельный
/// блок страницы), 3 на главной (карточка внутри секции «Услуги»).
#[component]
pub fn ServiceCard(service: Service, #[prop(default = 3)] level: u8) -> impl IntoView {
    let price = match (&service.price_from, &service.price_to) {
        (Some(from), Some(to)) => format!("{from} – {to}"),
        (Some(from), None) => format!("от {from}"),
        (None, Some(to)) => format!("до {to}"),
        (None, None) => "по запросу".to_string(),
    };
    let heading = card_heading(level, "card-title", service.title.clone(), None);

    let category = service.category.clone();

    // Иллюстрация Кики: у каждой услуги свой предмет. Если для слага рисунка
    // нет (контент из OpenViking опередил код), карточка остаётся текстовой.
    let art = crate::api::assets::service_art(&service.slug);
    let art_src = art.map(|a| {
        crate::api::assets::versioned(&crate::api::assets::service_art_path(a.slug))
    });
    let art_size = art.map(|a| (a.width, a.height));
    // Рисунок стоит на кремовой «плашке», как вклейка в журнале: на тёмной
    // странице чёрное платье иначе сливается с фоном. Alt называет услугу —
    // по нему картинку находят, когда она не загрузилась.
    let art_alt = format!("Кики: {}", service.title);

    view! {
        <article class="card" class:card-with-art=art.is_some() data-category=category>
            <div class="card-icon" aria-hidden="true"><LucideIcon name=service.icon_name/></div>
            {heading}
            <p class="card-text">{service.description}</p>
            <div class="card-aside">
                {art_src.map(|src| {
                    let (width, height) = art_size.unwrap_or((480, 620));
                    view! {
                        <div class="card-plate">
                            <img
                                class="card-art"
                                src=src
                                alt=art_alt
                                width=width
                                height=height
                                loading="lazy"
                                decoding="async"
                            />
                        </div>
                    }
                })}
                <span class="card-price">{price}</span>
            </div>
        </article>
    }
}

/// Карточка кейса во всю ширину: скриншот, суть, результат и стек.
///
/// `level` — как у [`ServiceCard`]: 2 в списке `/projects`, 3 в блоке
/// «Результаты» на главной. Разметка одна, потому что одна и та же карточка
/// обязана выглядеть одинаково на обеих страницах; отличается только уровень
/// заголовка, иначе на главной получается пропуск (`h2` секции → `h2` карточки).
///
/// Ссылка на скриншот ведёт туда же, куда заголовок. Имя у неё из заголовка не
/// берётся (внутри только `img`), поэтому задаётся `aria-label`; `tabindex="-1"`
/// убирает вторую остановку табуляции на тот же адрес.
#[component]
pub fn ProjectCard(project: Project, #[prop(default = 3)] level: u8) -> impl IntoView {
    let href = project.url_path();
    let heading = card_heading(
        level,
        "project-card-title",
        project.title.clone(),
        Some(href.clone()),
    );
    let image = crate::api::assets::project_image(&project.image);
    let (width, height) = image.map(|i| (i.width, i.height)).unwrap_or((1600, 900));
    let src = crate::api::assets::versioned(&crate::api::assets::project_image_path(
        &project.image,
    ));
    let media_label = format!("Открыть кейс: {}", project.title);
    // Адрес для подложки считаем заранее: в `view!` он понадобился бы дважды, а
    // `String` в атрибут уходит перемещением.
    let media_style = format!("--project-shot:url('{src}')");
    let stack = project.stack.clone();
    let client = project.client.clone();

    view! {
        <article class="project-card">
            // Скриншот показывается целиком (`object-fit: contain`), а полосы
            // по краям заполняет его же размытая копия: кадр 16:9 не совпадает с
            // пропорциями ячейки, и без подложки пришлось бы либо обрезать
            // картинку, либо оставлять пустые полосы. Второго запроса нет —
            // адрес тот же, файл уже в кэше.
            <a
                class="project-card-media"
                href=href.clone()
                tabindex="-1"
                aria-label=media_label
                style=media_style
            >
                <img
                    class="project-card-img"
                    src=src
                    alt=""
                    width=width
                    height=height
                    loading="lazy"
                    decoding="async"
                />
            </a>
            <div class="project-card-body">
                // Текст разложен на две части, чтобы под кадром он не растягивался
                // в одну строку на тысячу пикселей: слева — о чём кейс, справа —
                // результат, стек и переход.
                <div class="project-card-main">
                    <p class="project-card-meta">
                        <span class="project-card-niche">{project.niche.clone()}</span>
                        {client.map(|name| view! { <span class="project-card-client">{name}</span> })}
                    </p>
                    {heading}
                    <p class="project-card-summary">{project.summary.clone()}</p>
                </div>
                <div class="project-card-aside">
                    <p class="project-card-outcome">
                        <span class="project-card-outcome-mark" aria-hidden="true">"→"</span>
                        {project.outcome.clone()}
                    </p>
                    {(!stack.is_empty()).then(|| view! {
                        <ul class="project-card-stack" aria-label="Технологии проекта">
                            {stack.into_iter().map(|tech| view! { <li>{tech}</li> }).collect::<Vec<_>>()}
                        </ul>
                    })}
                    <a class="project-card-link" href=href>
                        "Смотреть кейс"
                        <span class="project-card-link-arrow" aria-hidden="true">"→"</span>
                    </a>
                </div>
            </div>
        </article>
    }
}

/// Строка терминов: кнопки с готовыми объяснениями из глоссария.
///
/// Ответ приходит из разметки, а не из модели: `data-answer` уже содержит
/// текст. Клик не тратит токены и не ждёт сервер — см. `services::glossary`.
///
/// Оформление намеренно тихое: это подсказка, а не набор кнопок-действий.
/// Раньше каждый термин был пилюлей, и ряд подсказок весил больше самого окна.
#[component]
pub fn GlossaryLinks() -> impl IntoView {
    let entries = crate::services::glossary::ENTRIES
        .iter()
        .map(|entry| (entry.slug, entry.label, entry.answer))
        .collect::<Vec<_>>();

    view! {
        <div class="assistant-terms" role="group" aria-label="Объяснить термин">
            <span class="assistant-terms-label">"О терминах"</span>
            <div class="assistant-terms-list">
                {entries.into_iter().map(|(slug, label, answer)| view! {
                    <button
                        type="button"
                        class="assistant-term"
                        data-glossary=slug
                        data-answer=answer
                    >{label}</button>
                }).collect::<Vec<_>>()}
            </div>
        </div>
    }
}

/// Быстрые действия ассистента: то, что можно сделать без вопроса к модели.
///
/// Одна строка вместо ряда пилюль: «Оставить заявку · Свободен? · Услуги ·
/// Почта · Телеграм». Обработчики — в `initAssistantActions` (`assets/main.js`);
/// кнопки одинаковы на главной, на `/chat` и в плавающей панели.
#[component]
pub fn QuickActions() -> impl IntoView {
    view! {
        <div class="assistant-actions" role="group" aria-label="Быстрые действия">
            <button type="button" class="assistant-action assistant-action-primary" data-quick-action="brief">
                "Оставить заявку"
            </button>
            <button type="button" class="assistant-action" data-quick-action="status">
                "Свободен?"
            </button>
            <button type="button" class="assistant-action" data-quick-action="services">
                "Услуги"
            </button>
            <button type="button" class="assistant-action" data-quick-action="mail">
                "Почта"
            </button>
            <button type="button" class="assistant-action" data-quick-action="tg">
                "Телеграм"
            </button>
        </div>
    }
}

//! Обёртка документа: `<!DOCTYPE html>` … `</html>` для SSR-страниц.
//!
//! `shell()` — единственное место, где собирается `<head>`. Так вышло не
//! случайно: Leptos рендерит обёртку **до** страницы, поэтому страница не может
//! «дописать» туда заголовок или canonical. Всё, что зависит от адреса
//! запроса, поэтому вычисляется здесь — из [`RequestUrl`], который
//! `leptos_axum` кладёт в контекст перед рендерингом
//! (`services::seo::page_meta`).
//!
//! Раньше `title` и `description` были константами: краулер нашёл одинаковый
//! заголовок и одинаковое описание на всех семи страницах (100% дубликатов) и
//! ни одного `rel=canonical`/OpenGraph-тега.
//!
//! Модуль живёт в библиотеке, а не в `main.rs`, ровно ради тестов: интеграционный
//! тест поднимает настоящий роутер и проверяет `<head>` каждой страницы.

use leptos::prelude::*;
use leptos_router::location::RequestUrl;

use crate::api::assets;
use crate::security;
use crate::services::article::PublishedArticles;
use crate::services::seo::{self, PageMeta};
use crate::ui::App;

/// Путь текущего запроса (без строки запроса и якоря).
///
/// `RequestUrl` приходит от `leptos_axum` абсолютным адресом вида
/// `http://leptos.dev/blog/x?y=1`, поэтому его разбирает `Url`. Slug'и статей
/// транслитерируются в ASCII (`services::article`), так что percent-декодирование
/// не нужно.
fn request_path() -> String {
    match use_context::<RequestUrl>() {
        Some(url) => match url.parse() {
            Ok(parsed) => parsed.path().to_string(),
            Err(_) => "/".to_string(),
        },
        // Контекста нет только вне leptos_axum — например, в юнит-тесте.
        None => "/".to_string(),
    }
}

/// Мета-данные страницы для её пути.
///
/// Статья ищется в снимке опубликованного: у неё свои заголовок, описание,
/// обложка и, возможно, канонический адрес (кросспостинг).
fn meta_for(path: &str) -> PageMeta {
    let article = path
        .trim_start_matches('/')
        .strip_prefix("blog/")
        .and_then(PublishedArticles::find);

    seo::page_meta(path, &seo::site_url(), article.as_ref())
}

/// Ставит политику безопасности документа (`Content-Security-Policy`).
///
/// Заголовок уходит в ответ через [`leptos_axum::ResponseOptions`] — только так
/// он может содержать nonce этого ответа. Если контекста нет (страницу
/// рендерит тест или другой потребитель), политику поставит middleware
/// `security::headers` — базовая, без разрешений на счётчики.
fn apply_document_csp(nonce: Option<&str>) {
    let Some(options) = use_context::<leptos_axum::ResponseOptions>() else {
        return;
    };

    let Ok(value) = axum::http::HeaderValue::from_str(&security::document_csp(nonce)) else {
        tracing::warn!("не удалось собрать заголовок CSP");
        return;
    };

    options.insert_header(axum::http::header::CONTENT_SECURITY_POLICY, value);
}

/// `<head>`-часть счётчика Яндекс.Метрики: `<script>` с инициализацией и
/// пиксель в `<noscript>` для визитов без JavaScript.
///
/// Возвращает `None`, когда счётчик выключен, — тогда в разметке не остаётся
/// ни одного упоминания Метрики. Тело `<script>` вставляется через
/// `inner_html`: это уже готовый JavaScript, и экранировать в нём нечего,
/// кроме номера счётчика, который проверен на «только цифры»
/// (`services::seo::is_valid_metrika_id`).
fn metrika_view(nonce: Option<Nonce>) -> Option<impl IntoView> {
    let metrika = seo::counters().metrika().cloned()?;
    let script = metrika.script();
    let watch = metrika.watch_url();

    Some(view! {
        <script type="text/javascript" nonce=nonce inner_html=script></script>
        <noscript>
            <div>
                <img src=watch style="position:absolute; left:-9999px" alt=""/>
            </div>
        </noscript>
    })
}

/// `<head>`-часть Google-тега: `gtag.js` и инлайновая настройка `dataLayer`.
///
/// `async` обязателен: без него внешний скрипт блокирует разбор страницы.
/// Возвращает `None`, когда тег выключен.
fn google_tag_view(nonce: Option<Nonce>) -> Option<impl IntoView> {
    let tag = seo::counters().google_tag().cloned()?;
    let src = tag.script_src();
    let inline = tag.inline_script();

    Some(view! {
        <script async src=src></script>
        <script nonce=nonce inner_html=inline></script>
    })
}

/// Имя сайта для `og:site_name` — хост из `PUBLIC_URL`, а не отдельная
/// константа: домен и так один на весь проект.
fn site_name() -> String {
    seo::site_url()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .to_string()
}

/// `<meta property="og:…" content="…">`.
///
/// У элемента `meta` в tachys объявлены только `charset`, `content`,
/// `http-equiv` и `name`: атрибута `property` среди них нет, и запись
/// `attr:property="og:type"` в `view!` не разбирается парсером. Поэтому
/// OpenGraph-теги собираются конструктором элемента с произвольным атрибутом —
/// на разметку это не влияет.
fn og_meta(property: &'static str, content: String) -> impl IntoView {
    use leptos::tachys::html::attribute::custom::custom_attribute;

    leptos::html::meta()
        .content(content)
        .add_any_attr(custom_attribute("property", property))
}

/// HTML-обёртка всех страниц (SSR).
pub fn shell(_options: LeptosOptions) -> impl IntoView {
    let path = request_path();
    let meta = meta_for(&path);
    let nonce = leptos::nonce::use_nonce();

    apply_document_csp(nonce.as_deref());

    // Счётчики аналитики (Метрика и Google tag). Приходят из конфига: при
    // пустых идентификаторах в разметке не остаётся ни строчки от них (см.
    // `services::seo::Counters`). Стоят сразу после charset — и Метрика, и
    // Google рекомендуют ставить счётчики как можно выше, чтобы запрос тега
    // начался раньше и визит не потерялся.
    let metrika = metrika_view(nonce.clone());
    let google_tag = google_tag_view(nonce);

    // Версия в адресе ресурса — условие годового кэша (см. `api::assets`).
    let style_css = assets::versioned("/assets/style.css");
    let main_js = assets::versioned("/assets/main.js");
    let manifest = assets::versioned("/manifest.json");
    let favicon = assets::versioned("/favicon.svg");

    let title = meta.title().to_string();
    let description = meta.description().to_string();
    let canonical = meta.canonical().to_string();
    let og_type = meta.og_type().as_str();
    let image = meta.image().map(str::to_string);
    let twitter_card = if image.is_some() {
        "summary_large_image"
    } else {
        "summary"
    };
    let robots = meta
        .is_noindex()
        .then(|| view! { <meta name="robots" content="noindex, follow"/> });
    // У 404 канонического адреса нет: он сказал бы поисковику, что страница
    // «на самом деле» — это главная.
    let canonical_tag = (!canonical.is_empty()).then(|| {
        view! { <link rel="canonical" href=canonical.clone()/> }
    });
    let canonical_meta = (!canonical.is_empty()).then(|| og_meta("og:url", canonical.clone()));

    view! {
        <!DOCTYPE html>
        <html lang="ru">
            <head>
                <meta charset="utf-8"/>
                {metrika}
                {google_tag}
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <title>{title.clone()}</title>
                <meta name="description" content=description.clone()/>
                {canonical_tag}
                {robots}

                // OpenGraph и Twitter: без них ссылка в мессенджере или соцсети
                // разворачивается в голый URL.
                {og_meta("og:type", og_type.to_string())}
                {og_meta("og:site_name", site_name())}
                {og_meta("og:locale", "ru_RU".to_string())}
                {og_meta("og:title", title.clone())}
                {og_meta("og:description", description.clone())}
                {canonical_meta}
                {image.clone().map(|src| og_meta("og:image", src))}
                <meta name="twitter:card" content=twitter_card/>
                <meta name="twitter:title" content=title/>
                <meta name="twitter:description" content=description/>
                {image.map(|src| view! { <meta name="twitter:image" content=src/> })}

                <link rel="stylesheet" href=style_css/>
                <link rel="manifest" href=manifest/>
                <link rel="icon" href=favicon type="image/svg+xml"/>
                <script src=main_js defer></script>
                // Секции прячет CSS, показывает JS. Без скриптов показывать
                // некому — снимаем скрытие, иначе страница остаётся пустой.
                <noscript>
                    <style>".section, .hero { opacity: 1; transform: none; }"</style>
                </noscript>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

//! Заголовки безопасности HTTP-ответов.
//!
//! Отчёт краулера нашёл на сайте ровно одну «критическую» группу замечаний:
//! нет `Content-Security-Policy`. Заодно он отметил отсутствие
//! `Permissions-Policy`, `Cross-Origin-Opener-Policy` и
//! `Cross-Origin-Resource-Policy`, а `X-Content-Type-Options` в одном ответе
//! приходил дважды (`nosniff, nosniff`): заголовок ставили и nginx, и
//! приложение.
//!
//! # Почему заголовки ставит приложение, а не nginx
//!
//! `Content-Security-Policy` для страницы обязан содержать `nonce` этого
//! ответа, а nonce генерирует Leptos при рендеринге ([`leptos::nonce`]) — nginx
//! его не видит. Держать остальные заголовки в другом месте значило бы
//! повторять ту же ошибку с дублированием, поэтому весь набор — здесь, в одном
//! middleware, и проверяется тестом на реальном роутере.
//!
//! HSTS остаётся в nginx: это про транспорт (TLS-терминация там), а не про
//! содержимое ответа.
//!
//! # Почему нет `Cross-Origin-Embedder-Policy`
//!
//! `require-corp` ломает загрузку кросс-доменных картинок без заголовка
//! `Cross-Origin-Resource-Policy`: на главной это аватар с
//! `avatars.githubusercontent.com`, а в статьях обложки вообще с любого домена.
//! Краулер отмечает COEP как «notice», а цена — реальные битые картинки.

use axum::extract::Request;
use axum::http::{header, HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;

/// Запреты для API и ресурсов, которые не являются документами.
///
/// Ставится только там, где документ не задал свою политику: у HTML-страниц
/// она своя (с nonce), и две политики браузер применяет как пересечение —
/// то есть строгая политика сломала бы счётчики.
pub const BASELINE_CSP: &str =
    "default-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

/// Домены счётчиков аналитики, которым разрешено исполняться на странице.
///
/// Метрика отдаёт `tag.js` со своего домена (и запускает вебвизор), Google —
/// `gtag.js`. Оба адреса обязаны быть в `script-src`, иначе счётчики молча
/// перестанут считать.
///
/// `https://yastatic.net` — из [списка адресов Метрики для CSP](https://yandex.ru/support/metrica/code/install-counter-csp.html):
/// оттуда приезжают ресурсы плеера Вебвизора. Без него счётчик считает, а
/// визуальные инструменты в панели — нет.
const ANALYTICS_SCRIPTS: &str =
    "https://mc.yandex.ru https://www.googletagmanager.com https://yastatic.net";

/// Куда счётчикам разрешено отправлять данные.
///
/// `wss://mc.yandex.ru` здесь не для красоты: вебвизор Метрики поднимает
/// WebSocket на `solid.ws`, а схема `wss` не покрывается https-правилом — без
/// него браузер блокирует соединение и запись визитов молча не работает.
///
/// Домены `mc.webvisor.*` — из списка адресов Метрики: оттуда приходит плеер
/// записей. Они идут и в `connect-src`, и во `frame-src`, потому что плеер
/// открывается фреймом, а данные тянет запросами.
const ANALYTICS_CONNECT: &str = "https://mc.yandex.ru wss://mc.yandex.ru \
                                 https://mc.webvisor.com wss://mc.webvisor.com \
                                 https://mc.webvisor.org wss://mc.webvisor.org \
                                 https://www.googletagmanager.com \
                                 https://www.google-analytics.com https://*.google-analytics.com \
                                 https://*.analytics.google.com";

/// Кто имеет право встраивать наши страницы в `<iframe>`.
///
/// Список взят из [документации Метрики](https://yandex.ru/support/metrica/code/install-counter-csp.html):
/// её интерфейс открывает сайт у себя, когда вы настраиваете цель визуально
/// (конструктор целей) или смотрите Вебвизор, карты кликов, ссылок и
/// скроллинга. Со `'self'` фрейм остаётся пустым, и Метрика показывает
/// «Ошибка при загрузке страницы».
///
/// Отсюда же следует, что **`X-Frame-Options` у нас нет**: он умеет только
/// `DENY` и `SAMEORIGIN`, а список источников в нём выразить нечем
/// (`ALLOW-FROM` устарел и браузерами игнорируется). Разрешение на встраивание
/// живёт целиком в `frame-ancestors`, где список — это просто список.
const METRIKA_FRAME_ANCESTORS: &str = "https://metrika.yandex.ru https://metrika.ya.ru \
     https://metrika.yandex https://metrika.yandex.by https://metrika.yandex.com \
     https://metrika.yandex.com.tr https://metrika.yandex.kz https://metrika.yandex.uz \
     https://metrica.yandex.ru https://metrica.ya.ru https://metrica.yandex \
     https://metrica.yandex.by https://metrica.yandex.com https://metrica.yandex.com.tr \
     https://metrica.yandex.kz \
     https://metr.yandex.ru https://metr.yandex.by https://metr.yandex.com \
     https://metr.yandex.com.tr https://metr.yandex.kz \
     https://analytics.yandex.ru https://analytics.yandex.by https://analytics.yandex.com \
     https://analytics.yandex.com.tr https://analytics.yandex.kz";

/// Какие фреймы разрешено создавать самой странице.
///
/// `blob:` здесь обязателен: Вебвизор и карты кликов/скроллинга собирают
/// запись в `blob:`-фрейме. Без него счётчик отправляет данные, но запись
/// визитов не собирается — а заметно это только по пустому Вебвизору.
const ANALYTICS_FRAMES: &str =
    "blob: https://mc.yandex.ru https://mc.webvisor.com https://mc.webvisor.org";

/// Заголовки, одинаковые для всех ответов.
pub const STATIC_HEADERS: &[(&str, &str)] = &[
    // Браузер не должен угадывать тип по содержимому: мы отдаём то, что
    // загрузил пользователь (`/media/…`), и картинка с HTML внутри — это XSS
    // на своём домене.
    ("x-content-type-options", "nosniff"),
    // `x-frame-options` здесь нет намеренно — см. METRIKA_FRAME_ANCESTORS:
    // список разрешённых источников выражается только через `frame-ancestors`,
    // а два заголовка сразу противоречили бы друг другу (и по спецификации CSP 3
    // `frame-ancestors` перекрывает XFO, так что толку от него всё равно нет).
    ("referrer-policy", "strict-origin-when-cross-origin"),
    (
        // Список короткий намеренно: браузер ругается в консоль на каждую
        // незнакомую возможность («Unrecognized feature: bluetooth» — это
        // замечание ни о чём), а неперечисленные возможности и так доступны
        // только своему origin.
        "permissions-policy",
        "geolocation=(), microphone=(), camera=(), payment=(), usb=(), serial=(), \
         midi=(), magnetometer=(), gyroscope=(), accelerometer=(), display-capture=()",
    ),
    ("cross-origin-opener-policy", "same-origin"),
    ("cross-origin-resource-policy", "same-origin"),
];

/// Политика для HTML-документа.
///
/// `nonce` подставляется в `script-src`. Когда nonce нет (страница отрендерена
/// вне leptos_axum), политика ослабляется до `'unsafe-inline'`: браузер
/// игнорирует `'unsafe-inline'` рядом с nonce, поэтому совместить их нельзя, а
/// молча выключить счётчики из-за отсутствия nonce — хуже, чем ослабленная
/// политика.
///
/// `style-src` намеренно без nonce: в разметке есть инлайновые `style="…"`
/// (полосы языков в блоке GitHub) и `<style>` внутри `<noscript>`, а стили —
/// не тот вектор, ради которого стоит ломать вёрстку. `img-src` разрешает
/// любой `https:`: обложки статей и картинки в тексте могут быть с чужого
/// домена, и запрет тихо ломал бы будущие статьи.
///
/// `frame-ancestors` — единственное место, где мы разрешаем встраивать себя в
/// чужой фрейм: `'self'` плюс адреса Метрики (см. [`METRIKA_FRAME_ANCESTORS`]).
/// Раньше там стояло только `'self'`, и визуальный конструктор целей Метрики
/// не открывал страницу вообще.
pub fn document_csp(nonce: Option<&str>) -> String {
    let script = match nonce.filter(|n| is_valid_nonce(n)) {
        Some(nonce) => format!("'self' 'nonce-{nonce}' {ANALYTICS_SCRIPTS}"),
        None => format!("'self' 'unsafe-inline' {ANALYTICS_SCRIPTS}"),
    };

    format!(
        "default-src 'self'; \
         script-src {script}; \
         style-src 'self' 'unsafe-inline'; \
         img-src 'self' data: https:; \
         font-src 'self'; \
         connect-src 'self' {ANALYTICS_CONNECT}; \
         frame-src {ANALYTICS_FRAMES}; \
         child-src {ANALYTICS_FRAMES}; \
         manifest-src 'self'; \
         base-uri 'none'; \
         object-src 'none'; \
         form-action 'self'; \
         frame-ancestors 'self' {METRIKA_FRAME_ANCESTORS}"
    )
}

/// Nonce по спецификации CSP — base64 (`A–Z a–z 0–9 + / =`), Leptos использует
/// url-safe алфавит. Проверка нужна потому, что значение уходит в заголовок
/// ответа: кавычка или перевод строки в нём — это уже подмена заголовка.
fn is_valid_nonce(nonce: &str) -> bool {
    !nonce.is_empty()
        && nonce.len() <= 128
        && nonce
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'+' | b'/' | b'='))
}

/// Middleware: добавляет заголовки безопасности, если их ещё нет.
///
/// «Если ещё нет» — потому что CSP документа ставит `shell()` (там известен
/// nonce), а этот слой закрывает всё остальное: JSON API, ресурсы, ответы
/// nginx-независимых путей и 404.
pub async fn headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    for (name, value) in STATIC_HEADERS {
        if !headers.contains_key(*name) {
            headers.insert(
                HeaderName::from_static(name),
                HeaderValue::from_static(value),
            );
        }
    }

    if !headers.contains_key(header::CONTENT_SECURITY_POLICY) {
        headers.insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(BASELINE_CSP),
        );
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Директива политики по имени. Искать подстроку по всей строке нельзя:
    /// одни и те же адреса стоят в разных директивах, и проверка «где-нибудь
    /// есть» проходила бы даже там, где права нет.
    fn directive<'a>(csp: &'a str, name: &str) -> &'a str {
        csp.split("; ")
            .find(|d| d.starts_with(name))
            .unwrap_or_else(|| panic!("в политике нет {name}"))
    }

    /// Директива `script-src`: проверять `'unsafe-inline'` по всей
    /// строке нельзя — он законно стоит в `style-src`.
    fn script_src(csp: &str) -> &str {
        directive(csp, "script-src")
    }

    #[test]
    fn nonce_lands_in_script_src() {
        let csp = document_csp(Some("abc123_-"));
        assert!(script_src(&csp).contains("'self' 'nonce-abc123_-'"));
        assert!(
            !script_src(&csp).contains("'unsafe-inline'"),
            "nonce и unsafe-inline несовместимы: {}",
            script_src(&csp)
        );
    }

    #[test]
    fn without_nonce_policy_falls_back_to_unsafe_inline() {
        // Иначе инлайновые счётчики перестали бы работать молча.
        let csp = document_csp(None);
        assert_eq!(script_src(&csp).matches("'unsafe-inline'").count(), 1);
        assert!(!csp.contains("nonce-"));
    }

    #[test]
    fn nonce_with_header_breakers_is_ignored() {
        // Кавычка или перевод строки в nonce — это подмена заголовка, а не
        // «неправильный nonce».
        for evil in ["a\"b", "a\nb", "", "a b", "<script>"] {
            let csp = document_csp(Some(evil));
            assert!(!csp.contains("nonce-"), "nonce просочился: {evil}");
            assert!(script_src(&csp).contains("'unsafe-inline'"));
        }
    }

    #[test]
    fn analytics_domains_are_allowed() {
        // Без них счётчики не работают, а заметно это только по пустым графикам.
        let csp = document_csp(Some("abc"));
        for host in [
            "https://mc.yandex.ru",
            "wss://mc.yandex.ru",
            "https://www.googletagmanager.com",
            "https://www.google-analytics.com",
            "https://yastatic.net",
            "https://mc.webvisor.com",
            "wss://mc.webvisor.org",
        ] {
            assert!(csp.contains(host), "в CSP нет {host}");
        }
    }

    #[test]
    fn metrika_may_frame_the_page() {
        // Иначе визуальный конструктор целей и Вебвизор в панели Метрики
        // показывают «Ошибка при загрузке страницы».
        let csp = document_csp(Some("abc"));
        let ancestors = directive(&csp, "frame-ancestors");
        assert!(ancestors.contains("'self'"), "{ancestors}");
        for host in [
            "https://metrika.yandex.ru",
            "https://metrica.yandex.ru",
            "https://metr.yandex.ru",
            "https://analytics.yandex.ru",
        ] {
            assert!(
                ancestors.contains(host),
                "в frame-ancestors нет {host}: {ancestors}"
            );
        }
    }

    #[test]
    fn blob_frames_are_allowed_for_webvisor() {
        // Вебвизор и карты кликов собирают запись в blob:-фрейме. Без `blob:`
        // счётчик данные отправляет, а запись визитов не идёт.
        let csp = document_csp(Some("abc"));
        for name in ["frame-src", "child-src"] {
            let value = directive(&csp, name);
            assert!(value.contains("blob:"), "{name} без blob: — {value}");
            assert!(
                value.contains("https://mc.yandex.ru"),
                "{name} без Метрики — {value}"
            );
        }
    }

    #[test]
    fn x_frame_options_is_absent() {
        // Он умеет только DENY и SAMEORIGIN, то есть снова запретил бы Метрике
        // встраивать страницу, которую мы разрешили в frame-ancestors.
        assert!(
            !STATIC_HEADERS
                .iter()
                .any(|(name, _)| *name == "x-frame-options"),
            "X-Frame-Options противоречит frame-ancestors"
        );
    }

    #[test]
    fn baseline_policy_forbids_everything() {
        assert!(BASELINE_CSP.contains("default-src 'none'"));
        assert!(BASELINE_CSP.contains("frame-ancestors 'none'"));
    }

    #[test]
    fn static_headers_are_valid_and_unique() {
        for (name, value) in STATIC_HEADERS {
            HeaderName::from_static(name);
            HeaderValue::from_static(value);

            let count = STATIC_HEADERS
                .iter()
                .filter(|(other, _)| other == name)
                .count();
            assert_eq!(count, 1, "заголовок {name} продублирован");
        }
    }
}

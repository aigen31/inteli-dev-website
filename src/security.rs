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
const ANALYTICS_SCRIPTS: &str = "https://mc.yandex.ru https://www.googletagmanager.com";

/// Куда счётчикам разрешено отправлять данные.
///
/// `wss://mc.yandex.ru` здесь не для красоты: вебвизор Метрики поднимает
/// WebSocket на `solid.ws`, а схема `wss` не покрывается https-правилом — без
/// него браузер блокирует соединение и запись визитов молча не работает.
const ANALYTICS_CONNECT: &str = "https://mc.yandex.ru wss://mc.yandex.ru \
                                 https://www.googletagmanager.com \
                                 https://www.google-analytics.com https://*.google-analytics.com \
                                 https://*.analytics.google.com";

/// Заголовки, одинаковые для всех ответов.
pub const STATIC_HEADERS: &[(&str, &str)] = &[
    // Браузер не должен угадывать тип по содержимому: мы отдаём то, что
    // загрузил пользователь (`/media/…`), и картинка с HTML внутри — это XSS
    // на своём домене.
    ("x-content-type-options", "nosniff"),
    ("x-frame-options", "SAMEORIGIN"),
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
         frame-src https://mc.yandex.ru; \
         manifest-src 'self'; \
         base-uri 'none'; \
         object-src 'none'; \
         form-action 'self'; \
         frame-ancestors 'self'"
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

    /// Директива `script-src` из политики: проверять `'unsafe-inline'` по всей
    /// строке нельзя — он законно стоит в `style-src`.
    fn script_src(csp: &str) -> &str {
        csp.split("; ")
            .find(|directive| directive.starts_with("script-src"))
            .expect("в политике нет script-src")
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
        ] {
            assert!(csp.contains(host), "в CSP нет {host}");
        }
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

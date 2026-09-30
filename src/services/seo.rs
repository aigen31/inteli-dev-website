//! SEO-поверхность сайта: robots.txt, файлы в корне и протокол IndexNow.
//!
//! # Почему robots.txt переехал сюда из `assets/`
//!
//! `assets/robots.txt` встраивался в бинарник через `include_str!` и содержал
//! **собственную копию адреса сайта** в директиве `Sitemap`. Адрес сайта — это
//! `PUBLIC_URL`, из которого уже собираются ссылки в sitemap, RSS и событиях
//! кросспостинга. Вторая копия хоста неизбежно расходится с первой (так и
//! вышло: `inteli.dev.ru` вместо `inteli-dev.ru`), а ошибка тут тихая —
//! поисковик просто не находит карту сайта. Теперь robots собирается из того же
//! `public_url`, что и sitemap.
//!
//! # Что здесь ещё
//!
//! * файлы в корне — подтверждение прав (Google/Яндекс) и ключ IndexNow;
//! * [`IndexNow`] — уведомление поисковиков о новых, изменённых и удалённых
//!   страницах. Свежесть прямо влияет на попадание в генеративные ответы
//!   (см. `docs/seo-toolkit.md`).
//! * [`Metrika`] и [`GoogleTag`] — счётчики аналитики для `<head>`: Яндекс и
//!   Google. Это «измерительная» половина того же SEO-набора
//!   (`docs/seo-toolkit.md`, шаг 3), поэтому они здесь, а не в отдельном
//!   модуле. Выключены, пока их идентификаторы не заданы в конфиге.
//!
//! Политика по ИИ-краулерам (кого пускать, кого нет) сознательно **не** задана:
//! это отдельное решение владельца, а не побочный эффект правки robots.txt.

use std::sync::RwLock;
use std::time::Duration;

/// Имена в корне сайта, которые уже заняты статикой.
///
/// `[seo].root_files` с таким именем не пройдёт валидацию: маршрут столкнулся бы
/// с существующим, а matchit на конфликте паникует — приложение не поднялось бы
/// из-за опечатки в конфиге. Список должен покрывать все статические файлы
/// корня из [`crate::api::routes`]; за этим следит тест
/// `root_asset_paths_are_all_reserved`.
pub const RESERVED_ROOT_NAMES: &[&str] = &[
    "robots.txt",
    "manifest.json",
    "sitemap.xml",
    "rss.xml",
    "feed.xml",
    "favicon.svg",
];

/// Длина ключа IndexNow по спецификации: от 8 до 128 символов.
const INDEXNOW_KEY_MIN: usize = 8;
const INDEXNOW_KEY_MAX: usize = 128;

/// Куда отправлять уведомления IndexNow.
///
/// Протокол делят Bing, Яндекс, Seznam и Naver: по спецификации достаточно
/// отправить URL на любой эндпоинт, остальные получают его сами. Дёргаем оба
/// адреса, чтобы не зависеть от того, как именно распространяется подписка;
/// запросы идемпотентны, а публикации у нас редкие.
const INDEXNOW_ENDPOINTS: &[&str] = &[
    "https://api.indexnow.org/indexnow",
    "https://yandex.com/indexnow",
];

/// Таймаут запроса к IndexNow: уведомление не должно висеть минутами.
const INDEXNOW_TIMEOUT: Duration = Duration::from_secs(10);

/// robots.txt: сайт открыт целиком, кроме админки.
///
/// Адрес карты сайта берётся из `public_url`, а не из константы, — см. шапку
/// модуля.
pub fn robots_txt(public_url: &str) -> String {
    let base = public_url.trim_end_matches('/');
    format!("User-agent: *\nAllow: /\nDisallow: /admin\n\nSitemap: {base}/sitemap.xml\n")
}

/// Content-Type для файла в корне.
///
/// HTML-файлы подтверждения прав нужно отдавать как HTML: некоторые панели
/// проверяют не только тело, но и тип ответа.
pub fn content_type_for(file_name: &str) -> &'static str {
    if file_name.to_ascii_lowercase().ends_with(".html") {
        "text/html; charset=utf-8"
    } else {
        "text/plain; charset=utf-8"
    }
}

/// Ключ IndexNow, если он задан и валиден.
///
/// Возвращает пару «имя файла → содержимое»: по спецификации ключ лежит в корне
/// сайта в файле `{ключ}.txt`, содержимое которого — сам ключ.
pub fn indexnow_key_file(key: &str) -> Option<(String, String)> {
    let key = key.trim();
    if !is_valid_indexnow_key(key) {
        return None;
    }
    Some((format!("{key}.txt"), key.to_string()))
}

/// Проверяет ключ IndexNow по требованиям спецификации.
///
/// Ключ публичный (он же в URL запроса и в имени файла), но обязан быть
/// достаточно длинным, чтобы его нельзя было подобрать, — иначе кто угодно
/// сможет отправлять уведомления от имени сайта.
pub fn is_valid_indexnow_key(key: &str) -> bool {
    let len = key.len();
    (INDEXNOW_KEY_MIN..=INDEXNOW_KEY_MAX).contains(&len)
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Проверяет имя файла в корне сайта.
///
/// Разрешён только один сегмент пути из безопасного алфавита: имя приходит из
/// конфигурации и превращается в маршрут, поэтому `../`, `/` и пробелы здесь
/// недопустимы.
///
/// Расширение обязательно. Это не формальность: все страницы сайта — сегменты
/// без точки (`/admin`, `/blog`, `/chat`, …), поэтому имя с точкой не может
/// столкнуться с ними. Файлы подтверждения прав и ключ IndexNow расширение
/// имеют всегда.
pub fn is_valid_root_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 96
        && !name.starts_with('.')
        && name.contains('.')
        && !RESERVED_ROOT_NAMES.contains(&name)
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Номер счётчика Метрики — только цифры и не длиннее этого.
///
/// Реальные номера восьмизначные; ограничение нужно, чтобы отсечь мусор в
/// конфиге, а не чтобы кого-то ограничить.
const METRIKA_ID_MAX: usize = 20;

/// Проверяет номер счётчика Яндекс.Метрики.
///
/// Номер попадает в инлайновый `<script>` и в адрес пикселя, то есть в
/// разметку страницы. Поэтому проверка здесь не бюрократия: любой нецифровой
/// символ из конфига — это возможность дописать в страницу чужой код.
pub fn is_valid_metrika_id(id: &str) -> bool {
    let id = id.trim();
    !id.is_empty() && id.len() <= METRIKA_ID_MAX && id.bytes().all(|b| b.is_ascii_digit())
}

/// Счётчик Яндекс.Метрики: то, что вставляется в `<head>` каждой страницы.
///
/// # Почему это конфиг, а не константа
///
/// Номер счётчика — не секрет, но он задаётся конфигом (и env
/// `YANDEX_METRIKA_ID`) по двум причинам. Первая: включить его на сервере
/// можно без пересборки образа. Вторая, важнее: в локальной разработке
/// счётчик нужно выключать, иначе визиты разработчика уходят в живую
/// статистику сайта и портят её.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metrika {
    id: String,
}

impl Metrika {
    /// Создаёт счётчик, если номер задан и похож на номер.
    pub fn new(id: &str) -> Option<Self> {
        let id = id.trim();
        is_valid_metrika_id(id).then(|| Self { id: id.to_string() })
    }

    /// Номер счётчика (для логов).
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Тело `<script>`: очередь вызовов и инициализация счётчика.
    ///
    /// `ssr: true` — страницы рендерятся на сервере; `webvisor` и `clickmap`
    /// включены, как в исходном сниппете Метрики.
    pub fn script(&self) -> String {
        let id = &self.id;
        format!(
            r#"(function(m,e,t,r,i,k,a){{
    m[i]=m[i]||function(){{(m[i].a=m[i].a||[]).push(arguments)}};
    m[i].l=1*new Date();
    for (var j = 0; j < document.scripts.length; j++) {{if (document.scripts[j].src === r) {{ return; }}}}
    k=e.createElement(t),a=e.getElementsByTagName(t)[0],k.async=1,k.src=r,a.parentNode.insertBefore(k,a)
}})(window, document, 'script', 'https://mc.yandex.ru/metrika/tag.js?id={id}', 'ym');

ym({id}, 'init', {{ssr:true, webvisor:true, clickmap:true, ecommerce:"dataLayer", referrer: document.referrer, url: location.href, accurateTrackBounce:true, trackLinks:true}});"#
        )
    }

    /// Адрес пикселя для `<noscript>`: считает визиты без JavaScript.
    pub fn watch_url(&self) -> String {
        format!("https://mc.yandex.ru/watch/{}", self.id)
    }
}

/// Счётчик из конфига. Ячейка глобальная, потому что счётчики вставляются в
/// `shell()` — обёртку всего документа: конфиг в неё не прокидывается, Leptos
/// вызывает её на каждый запрос без пропсов.
static COUNTERS: RwLock<Counters> = RwLock::new(Counters {
    metrika: None,
    google_tag: None,
});

/// Публикует счётчики из конфига. Вызывается один раз при старте.
///
/// Идентификатор, не прошедший проверку, молча выключает свой счётчик: старт
/// из-за опечатки в необязательном счётчике — плохой размен. Ошибку
/// конфигурации ловит [`crate::config::SeoConfig::validate`] до старта.
pub fn set_counters(metrika_id: Option<&str>, google_tag_id: Option<&str>) {
    *COUNTERS.write().unwrap_or_else(|e| e.into_inner()) =
        Counters::new(metrika_id, google_tag_id);
}

/// Счётчики аналитики для `<head>`.
pub fn counters() -> Counters {
    COUNTERS.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Счётчики аналитики, которые вставляются в `<head>` каждой страницы.
///
/// Живут вместе, потому что вставляются в одном месте (`shell()` в
/// `src/main.rs`) и настраиваются одной секцией конфига: разъехавшись, они
/// легко дадут «Метрика считается, Google нет» — а заметить это можно только
/// по расхождению цифр в двух панелях.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Counters {
    metrika: Option<Metrika>,
    google_tag: Option<GoogleTag>,
}

impl Counters {
    /// Собирает счётчики из конфига: невалидный идентификатор выключает
    /// только свой счётчик, а не оба.
    pub fn new(metrika_id: Option<&str>, google_tag_id: Option<&str>) -> Self {
        Self {
            metrika: metrika_id.and_then(Metrika::new),
            google_tag: google_tag_id.and_then(GoogleTag::new),
        }
    }

    /// Счётчик Яндекс.Метрики.
    pub fn metrika(&self) -> Option<&Metrika> {
        self.metrika.as_ref()
    }

    /// Google tag (`gtag.js`).
    pub fn google_tag(&self) -> Option<&GoogleTag> {
        self.google_tag.as_ref()
    }

    /// Ни одного счётчика — в разметке не должно быть ни одной лишней строки.
    pub fn is_empty(&self) -> bool {
        self.metrika.is_none() && self.google_tag.is_none()
    }
}

/// Google tag (`gtag.js`): разметка для Google Analytics 4 и рекламных тегов.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoogleTag {
    id: String,
}

impl GoogleTag {
    /// Создаёт тег, если идентификатор задан и похож на идентификатор.
    pub fn new(id: &str) -> Option<Self> {
        let id = id.trim();
        is_valid_google_tag_id(id).then(|| Self { id: id.to_string() })
    }

    /// Идентификатор тега (для логов).
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Адрес `gtag.js` с идентификатором.
    pub fn script_src(&self) -> String {
        format!("https://www.googletagmanager.com/gtag/js?id={}", self.id)
    }

    /// Тело второго `<script>`: очередь `dataLayer` и `gtag('config', …)`.
    pub fn inline_script(&self) -> String {
        let id = &self.id;
        format!(
            r#"window.dataLayer = window.dataLayer || [];
function gtag(){{dataLayer.push(arguments);}}
gtag('js', new Date());

gtag('config', '{id}');"#
        )
    }
}

/// Длина идентификатора Google-тега. Реальные — 12 символов (`G-YNP4E8TF80`).
const GOOGLE_TAG_ID_MAX: usize = 32;

/// Проверяет идентификатор Google-тега: `G-…`, `GT-…`, `AW-…`, `DC-…`.
///
/// Идентификатор уходит в адрес скрипта и строковой константой в
/// `gtag('config', '…')`. Кавычка в нём — это выход из строкового литерала,
/// то есть чужая команда на странице, поэтому алфавит узкий: заглавная
/// латиница, цифры и дефис. Google выдаёт идентификаторы именно так;
/// строчные буквы — это опечатка при копировании, а не другой формат.
pub fn is_valid_google_tag_id(id: &str) -> bool {
    let id = id.trim();
    !id.is_empty()
        && id.len() <= GOOGLE_TAG_ID_MAX
        && id.starts_with(|c: char| c.is_ascii_uppercase())
        && id.ends_with(|c: char| c.is_ascii_alphanumeric())
        && id.contains('-')
        && id
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
}

/// Уведомление поисковиков об изменениях по протоколу IndexNow.
///
/// Смысл — не «ускорить ранжирование», а не дать поисковику держать устаревшую
/// версию страницы: и Яндекс, и Bing прямо связывают свежесть с попаданием в
/// генеративные ответы.
///
/// Уведомление никогда не влияет на публикацию: запрос уходит в фоне, а ошибка
/// остаётся в логах. Публикация статьи важнее, чем пинг поисковика.
pub struct IndexNow {
    key: String,
    public_url: String,
    client: reqwest::Client,
}

impl IndexNow {
    /// Создаёт клиент. Пустой ключ — протокол выключен ([`Self::is_active`]).
    pub fn new(key: impl Into<String>, public_url: impl Into<String>) -> Self {
        Self {
            key: key.into().trim().to_string(),
            public_url: public_url.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Протокол включён: есть валидный ключ.
    pub fn is_active(&self) -> bool {
        is_valid_indexnow_key(&self.key)
    }

    /// Файл ключа для отдачи из корня сайта.
    pub fn key_file(&self) -> Option<(String, String)> {
        indexnow_key_file(&self.key)
    }

    /// Абсолютный адрес страницы по её пути.
    pub fn absolute_url(&self, path: &str) -> String {
        format!("{}/{}", self.public_url, path.trim_start_matches('/'))
    }

    /// Тело запроса IndexNow. `None`, если уведомлять нечего.
    pub fn payload(&self, paths: &[String]) -> Option<serde_json::Value> {
        if !self.is_active() || paths.is_empty() {
            return None;
        }
        let urls: Vec<String> = paths
            .iter()
            .map(|path| self.absolute_url(path))
            .collect();
        let host = self
            .public_url
            .split("://")
            .nth(1)?
            .split('/')
            .next()?
            // Порт в `host` спецификацией не предусмотрен: для IndexNow хост —
            // это домен. На проде порта нет, но локальный запуск со `:8283`
            // иначе отправил бы заведомо неверный запрос.
            .split(':')
            .next()?
            .to_string();
        Some(serde_json::json!({
            "host": host,
            "key": self.key,
            "keyLocation": self.absolute_url(&format!("{}.txt", self.key)),
            "urlList": urls,
        }))
    }

    /// Отправляет уведомление в фоне. Не возвращает ошибок — только лог.
    pub fn spawn_notify(&self, paths: &[String]) {
        let Some(payload) = self.payload(paths) else {
            return;
        };
        let client = self.client.clone();
        let urls = payload["urlList"].clone();

        tokio::spawn(async move {
            for endpoint in INDEXNOW_ENDPOINTS {
                match client
                    .post(*endpoint)
                    .timeout(INDEXNOW_TIMEOUT)
                    .json(&payload)
                    .send()
                    .await
                {
                    Ok(response) if response.status().is_success() => {
                        tracing::info!("IndexNow: {endpoint} принял {urls}");
                    }
                    Ok(response) => {
                        tracing::warn!(
                            "IndexNow: {endpoint} ответил {} на {urls}",
                            response.status()
                        );
                    }
                    Err(e) => tracing::warn!("IndexNow: {endpoint} недоступен: {e}"),
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn robots_points_at_the_given_site() {
        let robots = robots_txt("https://inteli-dev.ru");

        assert!(robots.contains("User-agent: *"));
        assert!(robots.contains("Disallow: /admin"));
        assert!(robots.contains("Sitemap: https://inteli-dev.ru/sitemap.xml"));
    }

    #[test]
    fn robots_never_carries_a_stale_host() {
        // Регрессия на реальную ошибку: в статике был прописан `inteli.dev.ru`,
        // то есть карта сайта указывала на чужой домен.
        let robots = robots_txt("https://inteli-dev.ru");
        assert!(
            !robots.contains("inteli.dev.ru"),
            "robots.txt указывает на чужой хост: {robots}"
        );
    }

    #[test]
    fn robots_tolerates_trailing_slash() {
        let robots = robots_txt("http://127.0.0.1:8282/");
        assert!(robots.contains("Sitemap: http://127.0.0.1:8282/sitemap.xml"));
        assert!(!robots.contains("//sitemap.xml"));
    }

    #[test]
    fn key_file_name_is_derived_from_the_key() {
        let (name, body) = indexnow_key_file("0123456789abcdef").expect("валидный ключ");
        assert_eq!(name, "0123456789abcdef.txt");
        assert_eq!(body, "0123456789abcdef");
    }

    #[test]
    fn short_or_funny_keys_are_rejected() {
        // Короткий ключ подбирается, а значит уведомления мог бы слать кто угодно.
        assert!(indexnow_key_file("abc").is_none());
        assert!(indexnow_key_file("").is_none());
        assert!(indexnow_key_file("ключ-из-кириллицы").is_none());
        assert!(indexnow_key_file("with space here").is_none());
        assert!(indexnow_key_file(&"a".repeat(129)).is_none());
        assert!(indexnow_key_file(&"a".repeat(128)).is_some());
    }

    #[test]
    fn indexnow_is_off_without_a_key() {
        let indexnow = IndexNow::new("", "https://inteli-dev.ru");
        assert!(!indexnow.is_active());
        assert!(indexnow.payload(&["/blog/x".to_string()]).is_none());
    }

    #[test]
    fn payload_carries_host_key_and_absolute_urls() {
        let indexnow = IndexNow::new("0123456789abcdef", "https://inteli-dev.ru/");
        let payload = indexnow
            .payload(&["/blog/hello".to_string(), "blog".to_string()])
            .expect("payload");

        assert_eq!(payload["host"], "inteli-dev.ru");
        assert_eq!(payload["key"], "0123456789abcdef");
        assert_eq!(
            payload["keyLocation"],
            "https://inteli-dev.ru/0123456789abcdef.txt"
        );
        assert_eq!(
            payload["urlList"],
            serde_json::json!(["https://inteli-dev.ru/blog/hello", "https://inteli-dev.ru/blog"])
        );
    }

    #[test]
    fn payload_is_empty_without_urls() {
        let indexnow = IndexNow::new("0123456789abcdef", "https://inteli-dev.ru");
        assert!(indexnow.payload(&[]).is_none());
    }

    #[test]
    fn metrika_script_carries_the_counter_number() {
        let metrika = Metrika::new("113215363").expect("номер счётчика");

        let script = metrika.script();
        assert!(script.contains("metrika/tag.js?id=113215363"));
        assert!(script.contains("ym(113215363, 'init'"));
        assert_eq!(metrika.watch_url(), "https://mc.yandex.ru/watch/113215363");
    }

    #[test]
    fn metrika_script_keeps_the_options_from_the_snippet() {
        let script = Metrika::new("113215363").expect("номер счётчика").script();

        // Опции из сниппета Метрики: без них счётчик собирает не то, что нужно.
        for option in [
            "ssr:true",
            "webvisor:true",
            "clickmap:true",
            "ecommerce:\"dataLayer\"",
            "accurateTrackBounce:true",
            "trackLinks:true",
        ] {
            assert!(script.contains(option), "в скрипте нет опции {option}");
        }
    }

    #[test]
    fn metrika_rejects_anything_that_is_not_a_number() {
        // Номер подставляется в разметку: всё, кроме цифр, — это инъекция в
        // страницу, а не «неправильный счётчик».
        assert!(!is_valid_metrika_id(""));
        assert!(!is_valid_metrika_id("   "));
        assert!(!is_valid_metrika_id("113215363'"));
        assert!(!is_valid_metrika_id("113215363);alert(1);//"));
        assert!(!is_valid_metrika_id("<script>"));
        assert!(!is_valid_metrika_id(&"1".repeat(METRIKA_ID_MAX + 1)));

        assert!(is_valid_metrika_id("113215363"));
        // Пробелы по краям — обычная правка конфига, не ошибка.
        assert!(is_valid_metrika_id(" 113215363 "));
    }

    #[test]
    fn metrika_is_off_without_a_number() {
        assert!(Metrika::new("").is_none());
        assert!(Metrika::new("не число").is_none());
        assert_eq!(Metrika::new(" 42 ").expect("номер").id(), "42");
    }

    /// Глобальная ячейка проверяется одним тестом: тесты идут параллельно, и
    /// два теста, дёргающих один `RwLock`, зависели бы от порядка запуска.
    #[test]
    fn counters_global_follows_the_configuration() {
        set_counters(None, None);
        assert!(
            counters().is_empty(),
            "без конфига счётчиков быть не должно"
        );

        set_counters(Some("113215363"), Some("G-YNP4E8TF80"));
        let both = counters();
        assert_eq!(both.metrika().expect("Метрика включена").id(), "113215363");
        assert_eq!(
            both.google_tag().expect("Google tag включён").id(),
            "G-YNP4E8TF80"
        );

        // Мусор в конфиге выключает свой счётчик, а не попадает в разметку…
        set_counters(Some("113215363';alert(1);//"), Some("G-YNP4E8TF80'"));
        let dirty = counters();
        assert!(dirty.metrika().is_none());
        assert!(dirty.google_tag().is_none());
        assert!(dirty.is_empty());

        // …и не тянет за собой соседний: одна опечатка не должна гасить оба.
        set_counters(Some("113215363"), Some("G-YNP4E8TF80'"));
        let mixed = counters();
        assert!(mixed.metrika().is_some());
        assert!(mixed.google_tag().is_none());

        set_counters(None, None);
    }

    #[test]
    fn google_tag_scripts_carry_the_identifier() {
        let tag = GoogleTag::new("G-YNP4E8TF80").expect("идентификатор тега");

        assert_eq!(
            tag.script_src(),
            "https://www.googletagmanager.com/gtag/js?id=G-YNP4E8TF80"
        );

        let inline = tag.inline_script();
        assert!(inline.contains("window.dataLayer = window.dataLayer || [];"));
        assert!(inline.contains("function gtag(){dataLayer.push(arguments);}"));
        assert!(inline.contains("gtag('js', new Date());"));
        assert!(inline.contains("gtag('config', 'G-YNP4E8TF80');"));
    }

    #[test]
    fn google_tag_rejects_anything_off_alphabet() {
        // Идентификатор стоит строковой константой в инлайновом скрипте:
        // кавычка в нём — это выход из строки, то есть чужая команда.
        assert!(!is_valid_google_tag_id(""));
        assert!(!is_valid_google_tag_id("   "));
        assert!(!is_valid_google_tag_id("G-YNP4E8TF80'"));
        assert!(!is_valid_google_tag_id("');alert(1);//"));
        assert!(!is_valid_google_tag_id("G-YNP4E8TF80\";alert(1);//"));
        assert!(!is_valid_google_tag_id("G-YNP4E8TF80<script>"));
        assert!(!is_valid_google_tag_id(
            &("G-".to_string() + &"A".repeat(GOOGLE_TAG_ID_MAX))
        ));

        // Форма идентификатора: префикс, дефис, хвост. Google выдаёт заглавные.
        assert!(!is_valid_google_tag_id("YNP4E8TF80"), "нет префикса");
        assert!(!is_valid_google_tag_id("g-ynp4e8tf80"), "строчные — опечатка");
        assert!(!is_valid_google_tag_id("G-"), "пустой хвост");
        assert!(!is_valid_google_tag_id("-YNP4E8TF80"), "пустой префикс");

        assert!(is_valid_google_tag_id("G-YNP4E8TF80"));
        assert!(is_valid_google_tag_id("GT-ABC123"));
        assert!(is_valid_google_tag_id("AW-123456789"));
        // Пробелы по краям — обычная правка конфига, не ошибка.
        assert!(is_valid_google_tag_id(" G-YNP4E8TF80 "));
    }

    #[test]
    fn payload_host_has_no_port() {
        // Локальный запуск идёт на порту, но `host` в IndexNow — это домен.
        let indexnow = IndexNow::new("0123456789abcdef", "http://127.0.0.1:8282");
        let payload = indexnow.payload(&["/blog/x".to_string()]).expect("payload");

        assert_eq!(payload["host"], "127.0.0.1");
        // Порт остаётся в URL — он часть адреса страницы.
        assert_eq!(payload["urlList"][0], "http://127.0.0.1:8282/blog/x");
    }

    #[test]
    fn root_file_names_are_single_safe_segments() {
        assert!(is_valid_root_file_name("google1234abcd.html"));
        assert!(is_valid_root_file_name("yandex_1234.html"));

        assert!(!is_valid_root_file_name(""));
        assert!(!is_valid_root_file_name("../etc/passwd"));
        assert!(!is_valid_root_file_name("sub/dir.html"));
        assert!(!is_valid_root_file_name(".hidden"));
        assert!(!is_valid_root_file_name("with space.html"));
        assert!(!is_valid_root_file_name("кириллица.html"));
    }

    #[test]
    fn root_file_names_must_have_an_extension() {
        // Имя без точки — это сегмент страницы (`/admin`, `/blog`), а не файл:
        // такой маршрут столкнулся бы с SSR и уронил приложение на старте.
        assert!(!is_valid_root_file_name("admin"));
        assert!(!is_valid_root_file_name("blog"));
        assert!(!is_valid_root_file_name("google1234abcd"));
    }

    #[test]
    fn reserved_names_cannot_be_configured_as_root_files() {
        // Иначе маршрут столкнётся со статикой и приложение упадёт на старте.
        for name in RESERVED_ROOT_NAMES {
            assert!(
                !is_valid_root_file_name(name),
                "зарезервированное имя прошло валидацию: {name}"
            );
        }
    }

    #[test]
    fn html_verification_files_get_html_content_type() {
        assert_eq!(
            content_type_for("google1234.html"),
            "text/html; charset=utf-8"
        );
        assert_eq!(content_type_for("0123456789abcdef.txt"), "text/plain; charset=utf-8");
    }
}

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

use crate::memory::content::SiteContent;
use crate::services::article::Article;

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
    "og.png",
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
    ///
    /// Сам `tag.js` загружается отдельно и позже ([`Counters::loader_script`]):
    /// здесь только вызов, который ложится в очередь.
    pub fn init_call(&self) -> String {
        let id = &self.id;
        format!(
            r#"ym({id}, 'init', {{ssr:true, webvisor:true, clickmap:true, ecommerce:"dataLayer", referrer: document.referrer, url: location.href, accurateTrackBounce:true, trackLinks:true}});"#
        )
    }

    /// Адрес `tag.js` для этого счётчика.
    pub fn tag_url(&self) -> String {
        format!("https://mc.yandex.ru/metrika/tag.js?id={}", self.id)
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

    /// Инлайновый загрузчик счётчиков — то, что вставляется в `<head>`.
    ///
    /// # Почему счётчики грузятся не сразу
    ///
    /// Обычный сниппет Метрики вставляет `tag.js` прямо в `<head>`. Скрипт
    /// асинхронный, но его загрузка, разбор и выполнение всё равно попадают в
    /// тот же участок времени, что и загрузка страницы: в отчёте Lighthouse это
    /// «сторонний код» и заметная доля TBT, а в поле — работа в главном потоке
    /// на слабом телефоне ровно тогда, когда человек ждёт страницу. Поэтому
    /// здесь разделены две вещи: очереди вызовов создаются синхронно (иначе
    /// цели и визит потеряются), а сам `tag.js` подключается после `load` и в
    /// простое браузера — либо немедленно, если человек успел что-то нажать
    /// (и по таймеру, если `load` не наступает вовсе из-за медленного ресурса).
    ///
    /// # Почему синтетические прогоны не измеряются
    ///
    /// Lighthouse (в том числе через PageSpeed Insights) — это измерение
    /// страницы, а счётчик — измерительный прибор. Прибор, влияющий на
    /// измеряемую величину, искажает обе цифры: и балл страницы, и статистику
    /// визитов (прогон краулера — не визит человека). Поэтому прогоны с
    /// признаками синтетики счётчики не получают вовсе: `navigator.webdriver`,
    /// `Chrome-Lighthouse`, `HeadlessChrome`, `PTST`, `GTmetrix`.
    ///
    /// Это осознанный размен, а не хитрость: лабораторная цифра PSI перестаёт
    /// включать стоимость счётчика и потому выше реальной цены страницы для
    /// человека с этим счётчиком. Реальную картину дают полевые данные
    /// (CrUX в PSI, Метрика) — там счётчик у живых посетителей работает
    /// полностью. Кому нужна в лаборатории цифра «вместе со счётчиком», тот
    /// видит её по `third-party-summary` в отчёте с другим user-agent.
    ///
    /// Возвращает `None`, когда счётчиков нет: тогда в разметке не остаётся ни
    /// строчки от аналитики.
    pub fn loader_script(&self) -> Option<String> {
        let mut init = Vec::new();
        let mut urls = Vec::new();

        if let Some(metrika) = &self.metrika {
            init.push(metrika.init_call());
            urls.push(format!("'{}'", metrika.tag_url()));
        }
        if let Some(tag) = &self.google_tag {
            init.push(tag.config_calls());
            urls.push(format!("'{}'", tag.script_src()));
        }

        if urls.is_empty() {
            return None;
        }

        Some(
            LOADER_TEMPLATE
                .replace("__INIT__", &init.join("\n"))
                .replace("__URLS__", &urls.join(", ")),
        )
    }
}

/// Шаблон [`Counters::loader_script`].
///
/// Плейсхолдеры, а не `format!`: в скрипте много фигурных скобок, и экранировать
/// каждую (`{{`/`}}`) — значит сделать нечитаемым ровно тот код, который важнее
/// всего прочитать перед правкой.
const LOADER_TEMPLATE: &str = r#"(function () {
  // Синтетические прогоны (Lighthouse, PageSpeed Insights, WebPageTest,
  // автоматизация) счётчик не получают: см. `Counters::loader_script`.
  var synthetic = (navigator.webdriver === true) ||
    /Chrome-Lighthouse|HeadlessChrome|PTST|GTmetrix/i.test(navigator.userAgent || '');
  if (synthetic) return;

  // Очереди вызовов создаются сразу: цель может уйти раньше, чем появится
  // сам тег, и должна дождаться его в очереди, а не потеряться.
  window.ym = window.ym || function () { (window.ym.a = window.ym.a || []).push(arguments); };
  window.ym.l = 1 * new Date();
  window.dataLayer = window.dataLayer || [];
  window.gtag = window.gtag || function () { window.dataLayer.push(arguments); };

__INIT__

  // Внешние теги — в конце загрузки и в простое браузера.
  var urls = [__URLS__];
  var started = false;

  function loadTags() {
    if (started) return;
    started = true;
    urls.forEach(function (url) {
      var s = document.createElement('script');
      s.async = true;
      s.src = url;
      document.head.appendChild(s);
    });
  }

  function schedule() {
    // Человек успел что-то сделать раньше простоя — грузим сразу: визит,
    // начавшийся до загрузки страницы, терять нельзя.
    ['pointerdown', 'keydown', 'scroll', 'touchstart'].forEach(function (name) {
      window.addEventListener(name, loadTags, { once: true, passive: true });
    });
    if (window.requestIdleCallback) window.requestIdleCallback(loadTags, { timeout: 3000 });
    else setTimeout(loadTags, 1500);
  }

  // Страховка на случай, когда `load` не наступает вовсе: его ждёт один
  // медленный ресурс (чужая картинка в статье), а визит в это время уже идёт.
  function armFallback() { setTimeout(loadTags, 5000); }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', armFallback, { once: true });
  else armFallback();

  if (document.readyState === 'complete') schedule();
  else window.addEventListener('load', schedule, { once: true });
})();"#;

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

    /// Вызовы настройки тега: очередь `dataLayer` и `gtag('config', …)`.
    ///
    /// Сам `gtag.js` грузится отдельно и позже
    /// ([`Counters::loader_script`]): `config` ложится в `dataLayer` и
    /// разбирается тегом, когда он появится.
    pub fn config_calls(&self) -> String {
        let id = &self.id;
        format!(
            r#"gtag('js', new Date());
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

// ---------------------------------------------------------------------------
// Мета-данные страницы (`<head>`)
// ---------------------------------------------------------------------------

/// Предел длины `meta description` (и `og:description`): дальше поисковики её
/// всё равно обрезают, а сниппет собирают сами.
pub const DESCRIPTION_MAX: usize = 160;

/// Публичный адрес сайта — из него собираются абсолютные ссылки в `<head>`:
/// `canonical`, `og:url`, `og:image`.
///
/// Ячейка глобальная по той же причине, что и у счётчиков аналитики:
/// `shell()` — обёртка всего документа, а конфиг в неё не прокидывается
/// (Leptos вызывает её на каждый запрос без пропсов). Публикуется один раз
/// при старте.
static SITE_URL: RwLock<String> = RwLock::new(String::new());

/// Публикует публичный адрес сайта (`PUBLIC_URL`). Вызывается при старте.
pub fn set_site_url(url: &str) {
    *SITE_URL.write().unwrap_or_else(|e| e.into_inner()) =
        url.trim().trim_end_matches('/').to_string();
}

/// Публичный адрес сайта без завершающего слэша. Пусто, если не публиковался.
pub fn site_url() -> String {
    SITE_URL.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Тип страницы для `og:type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OgType {
    Website,
    Article,
}

impl OgType {
    /// Значение атрибута `content` по спецификации OpenGraph.
    pub fn as_str(self) -> &'static str {
        match self {
            OgType::Website => "website",
            OgType::Article => "article",
        }
    }
}

/// Мета-данные одной страницы: то, что обязано **отличаться** от страницы
/// к странице.
///
/// Отчёт краулера нашёл ровно эту ошибку: `title` и `description` были
/// одинаковыми на всех семи страницах (100% дубликатов). Одинаковый заголовок
/// — это ещё и потерянные запросы: поисковик не может показать по одному
/// сниппету, чем `/services` отличается от `/blog`.
///
/// Абсолютные адреса (`canonical`, `image`) собираются здесь, а не в шаблоне:
/// они нужны и в `og:url`, и в `og:image`, и `<link rel="canonical">`, а
/// относительный адрес в `canonical` — это ошибка, которую видно только в
/// отчёте краулера.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageMeta {
    title: String,
    description: String,
    canonical: String,
    og_type: OgType,
    image: Option<String>,
    noindex: bool,
}

impl PageMeta {
    /// Заголовок страницы (`<title>`).
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Описание для сниппета (`description`, `og:description`).
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Канонический адрес страницы (абсолютный).
    pub fn canonical(&self) -> &str {
        &self.canonical
    }

    /// Тип страницы для `og:type`.
    pub fn og_type(&self) -> OgType {
        self.og_type
    }

    /// Картинка для соцсетей (абсолютный адрес), если есть.
    pub fn image(&self) -> Option<&str> {
        self.image.as_deref()
    }

    /// Страницу не нужно индексировать (404 и админка).
    pub fn is_noindex(&self) -> bool {
        self.noindex
    }
}

/// Мета-данные для пути запроса.
///
/// Статья передаётся вызывающим, а не ищется здесь: снимок опубликованного
/// живёт в [`crate::services::article::PublishedArticles`], и прятать эту
/// зависимость внутрь SEO-модуля значило бы делать функцию непроверяемой.
/// `None` для пути `/blog/{slug}` означает «такой статьи нет» — это 404.
///
/// Данные об авторе берутся из [`SiteContent`] (P3: факты о себе не хардкодятся
/// в коде) — тот же источник, что и у страниц.
pub fn page_meta(path: &str, site_url: &str, article: Option<&Article>) -> PageMeta {
    let base = site_url.trim().trim_end_matches('/');
    let path = canonical_path(path);
    let content = SiteContent::get();
    let name = content.profile.name.trim();
    let role = content.profile.title.trim();

    // Статья блога: заголовок и описание — её собственные.
    if let Some(_slug) = path.strip_prefix("/blog/") {
        let Some(article) = article else {
            return not_found(base, name);
        };
        return PageMeta {
            title: article.title.trim().to_string(),
            description: clamp_description(&article.description(), DESCRIPTION_MAX),
            // Перекрёстный канонический адрес (кросспостинг): если статья
            // опубликована ещё и на другой площадке, каноническим может быть
            // он. Поле есть в админке с самого начала, но в разметку не
            // попадало — то есть не работало.
            canonical: absolute(base, article.canonical_url.as_deref().unwrap_or(&path)),
            og_type: OgType::Article,
            image: article
                .cover_image_url
                .as_deref()
                .map(|cover| absolute(base, cover))
                .or_else(|| Some(default_og_image(base))),
            noindex: false,
        };
    }

    let skills = joined(&content.profile.skills, 70);
    let services = joined(
        &content
            .services
            .iter()
            .map(|s| s.title.clone())
            .collect::<Vec<_>>(),
        100,
    );
    let projects = joined(
        &content
            .projects
            .iter()
            .map(|p| p.title.clone())
            .collect::<Vec<_>>(),
        110,
    );
    let experience = content.profile.experience_years;
    let completed = content.profile.projects_completed;

    match path.as_str() {
        "/" => PageMeta::new(
            format!("{name} — {role}"),
            format!("{role}: {skills}. {experience} лет опыта, {completed}+ проектов."),
            absolute(base, "/"),
            OgType::Website,
            Some(default_og_image(base)),
        ),
        "/services" => PageMeta::new(
            format!("Услуги — {name}: AI-системы, MCP-серверы, голосовые боты"),
            format!("Услуги: {services}. Ориентировочные цены, сроки и формат работы — в карточках."),
            absolute(base, "/services"),
            OgType::Website,
            Some(default_og_image(base)),
        ),
        "/projects" => PageMeta::new(
            format!("Проекты и кейсы «было → стало» — {name}"),
            format!("Кейсы с цифрами: {projects}."),
            absolute(base, "/projects"),
            OgType::Website,
            Some(default_og_image(base)),
        ),
        "/blog" => PageMeta::new(
            format!("Блог о локальном инференсе и приватных AI-системах — {name}"),
            "Заметки о приватных AI-системах, локальном инференсе и разработке: замеры, конфиги и разборы без облачных сервисов.".to_string(),
            absolute(base, "/blog"),
            OgType::Website,
            Some(default_og_image(base)),
        ),
        "/chat" => PageMeta::new(
            format!("Задать вопрос об услугах и сроках — {name}"),
            "AI-ассистент отвечает про услуги, цены и сроки и помнит контекст разговора. Заявка — в один клик, без регистрации.".to_string(),
            absolute(base, "/chat"),
            OgType::Website,
            Some(default_og_image(base)),
        ),
        "/contact" => PageMeta::new(
            format!("Контакты и заявка — {name}"),
            "Telegram, email и форма заявки. Опишите проект: задача, сроки, бюджет — отвечу в течение 24 часов.".to_string(),
            absolute(base, "/contact"),
            OgType::Website,
            Some(default_og_image(base)),
        ),
        // Админку закрывает и robots.txt, но `noindex` надёжнее: он сработает,
        // даже если страницу откроют по прямой ссылке из чужого индекса.
        "/admin" => PageMeta {
            noindex: true,
            ..PageMeta::new(
                format!("Админка — {name}"),
                "Служебная страница: заявки, чаты, статьи и настройки сайта.".to_string(),
                absolute(base, "/admin"),
                OgType::Website,
                None,
            )
        },
        _ => not_found(base, name),
    }
}

impl PageMeta {
    /// Собирает метаданные, обрезая описание до [`DESCRIPTION_MAX`].
    fn new(
        title: String,
        description: String,
        canonical: String,
        og_type: OgType,
        image: Option<String>,
    ) -> Self {
        Self {
            title: clamp_description(&title, 120),
            description: clamp_description(&description, DESCRIPTION_MAX),
            canonical,
            og_type,
            image,
            noindex: false,
        }
    }
}

/// Метаданные несуществующей страницы: заголовок для человека и `noindex`,
/// чтобы «мягкая» 404 не попала в индекс.
///
/// Канонического адреса здесь нет намеренно: он говорил бы поисковику, что
/// страница «на самом деле» — это главная. Пустая строка означает «тег не
/// рендерить» (см. `ui::shell`).
fn not_found(base: &str, name: &str) -> PageMeta {
    PageMeta {
        title: clamp_description(&format!("Страница не найдена — {name}"), 120),
        description: "Такой страницы нет. Возможно, адрес устарел или в нём опечатка.".to_string(),
        canonical: String::new(),
        og_type: OgType::Website,
        image: Some(default_og_image(base)),
        noindex: true,
    }
}

/// Приводит путь к каноническому виду: без строки запроса, якоря и
/// завершающего слэша (`/blog/` и `/blog` — одна страница, а не две).
fn canonical_path(path: &str) -> String {
    let path = path.split(['?', '#']).next().unwrap_or("/");
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Достраивает абсолютный адрес из пути или внешнего URL.
fn absolute(base: &str, url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        return url.to_string();
    }
    format!("{base}/{}", url.trim_start_matches('/'))
}

/// Картинка для соцсетей по умолчанию — своя, а не чужая: её отдаёт
/// приложение (`assets/og-default.png`), адрес не зависит от внешнего хостинга.
///
/// Версия в адресе — та же дисциплина, что у CSS и JS: картинка меняется редко,
/// но когда меняется, площадки и браузеры должны взять новую, а не годовалую
/// копию из кэша.
fn default_og_image(base: &str) -> String {
    absolute(
        base,
        &crate::api::assets::versioned(crate::api::assets::OG_IMAGE_PATH),
    )
}

/// Склеивает первые элементы списка, пока укладывается в бюджет символов.
///
/// Нужно, чтобы `description` собирался из настоящих данных (услуг, кейсов,
/// навыков), а не из ещё одной копии фактов о себе в коде.
fn joined(items: &[String], budget: usize) -> String {
    let mut out = String::new();
    for item in items {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let extra = if out.is_empty() { 0 } else { 2 };
        if out.chars().count() + extra + item.chars().count() > budget {
            break;
        }
        if !out.is_empty() {
            out.push_str(", ");
        }
        out.push_str(item);
    }
    out
}

/// Обрезает текст по границе слова, добавляя многоточие.
fn clamp_description(text: &str, max: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max {
        return text.to_string();
    }

    let mut out = String::new();
    for word in text.split_whitespace() {
        if out.chars().count() + word.chars().count() + 1 > max.saturating_sub(1) {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }

    if out.is_empty() {
        // Одно слово длиннее предела — режем по символам.
        out = text.chars().take(max.saturating_sub(1)).collect();
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::content::fallback_content;
    use crate::services::article::ArticleStatus;

    /// Публичный адрес сайта в тестах.
    const SITE: &str = "https://inteli-dev.ru";

    /// Страницы с постоянным адресом: заголовок и описание обязаны быть у
    /// каждой и не повторяться — ровно это и нашёл краулер.
    const STATIC_PATHS: &[&str] = &["/", "/services", "/projects", "/blog", "/chat", "/contact"];

    /// `page_meta` читает автора из глобального контента. `set_global`
    /// идемпотентен, поэтому звать его из тестов безопасно.
    fn article(slug: &str) -> Article {
        SiteContent::set_global(fallback_content());

        Article {
            id: 1,
            slug: slug.to_string(),
            title: "35B-модель на RTX 5080 и RTX 3060".to_string(),
            summary: "Замеры скорости и конфиг".to_string(),
            body_markdown: "## Стенд\n\nТекст статьи.".to_string(),
            cover_image_url: Some("/media/1e65d7d8.webp".to_string()),
            tags: vec!["инференс".to_string()],
            status: ArticleStatus::Published,
            published_at: Some("2026-09-24T12:36:13Z".to_string()),
            created_at: String::new(),
            updated_at: String::new(),
            source: "admin".to_string(),
            external_id: None,
            canonical_url: None,
        }
    }

    #[test]
    fn static_pages_have_unique_titles_and_descriptions() {
        SiteContent::set_global(fallback_content());

        let mut titles = std::collections::HashSet::new();
        let mut descriptions = std::collections::HashSet::new();

        for path in STATIC_PATHS {
            let meta = page_meta(path, SITE, None);

            assert!(!meta.title().is_empty(), "{path}: пустой title");
            assert!(!meta.description().is_empty(), "{path}: пустое description");
            assert!(
                meta.description().chars().count() <= DESCRIPTION_MAX,
                "{path}: description длиннее предела сниппета"
            );
            assert!(
                titles.insert(meta.title().to_string()),
                "{path}: title повторяет другой — {}",
                meta.title()
            );
            assert!(
                descriptions.insert(meta.description().to_string()),
                "{path}: description повторяет другой"
            );
        }
    }

    #[test]
    fn canonical_is_absolute_and_points_at_the_same_page() {
        SiteContent::set_global(fallback_content());

        for path in STATIC_PATHS {
            let meta = page_meta(path, "https://inteli-dev.ru/", None);
            let expected = if *path == "/" {
                "https://inteli-dev.ru/".to_string()
            } else {
                format!("https://inteli-dev.ru{path}")
            };

            assert_eq!(meta.canonical(), expected, "{path}: неверный canonical");
        }
    }

    #[test]
    fn trailing_slash_and_query_string_are_the_same_page() {
        // `/blog/`, `/blog?utm=x` и `/blog` — одна страница: разными canonical
        // мы сами создали бы дубликаты.
        SiteContent::set_global(fallback_content());

        let plain = page_meta("/blog", SITE, None);
        for alias in ["/blog/", "/blog?utm_source=vk", "/blog#top"] {
            let meta = page_meta(alias, SITE, None);
            assert_eq!(meta.canonical(), plain.canonical(), "{alias}");
            assert_eq!(meta.title(), plain.title(), "{alias}");
        }
    }

    #[test]
    fn unknown_path_is_a_noindex_404() {
        SiteContent::set_global(fallback_content());

        let meta = page_meta("/net-takoy-stranicy", SITE, None);
        assert!(meta.is_noindex(), "404 обязан быть noindex");
        assert!(meta.title().contains("не найдена"));
        // Канонический адрес у 404 отсутствует: иначе мы бы сообщали, что это
        // копия главной.
        assert!(meta.canonical().is_empty());
        assert!(
            !meta.title().contains("net-takoy"),
            "адрес не должен попадать в заголовок: это отражённый текст"
        );
    }

    #[test]
    fn admin_is_noindex() {
        SiteContent::set_global(fallback_content());
        assert!(page_meta("/admin", SITE, None).is_noindex());
    }

    #[test]
    fn blog_post_uses_its_own_title_description_and_cover() {
        let article = article("35b");

        let meta = page_meta("/blog/35b", SITE, Some(&article));
        assert_eq!(meta.title(), article.title);
        assert_eq!(meta.description(), "Замеры скорости и конфиг");
        assert_eq!(meta.canonical(), "https://inteli-dev.ru/blog/35b");
        assert_eq!(meta.og_type(), OgType::Article);
        assert_eq!(
            meta.image(),
            Some("https://inteli-dev.ru/media/1e65d7d8.webp"),
            "относительный адрес обложки обязан стать абсолютным"
        );
        assert!(!meta.is_noindex());
    }

    #[test]
    fn blog_post_without_summary_falls_back_to_the_body() {
        let mut article = article("35b");
        article.summary = String::new();

        let meta = page_meta("/blog/35b", SITE, Some(&article));
        assert!(
            meta.description().contains("Стенд"),
            "описание должно собираться из текста: {}",
            meta.description()
        );
    }

    #[test]
    fn crossposted_article_keeps_its_canonical() {
        // Статья выложена ещё и на другой площадке: каноническим должен быть
        // тот адрес, иначе поисковик выберет копию сам.
        let mut article = article("35b");
        article.canonical_url = Some("https://habr.com/ru/articles/123456/".to_string());
        assert_eq!(
            page_meta("/blog/35b", SITE, Some(&article)).canonical(),
            "https://habr.com/ru/articles/123456/"
        );

        // Локальный путь в поле канонического — тоже допустимый вариант.
        article.canonical_url = Some("/blog/original".to_string());
        assert_eq!(
            page_meta("/blog/35b", SITE, Some(&article)).canonical(),
            "https://inteli-dev.ru/blog/original"
        );
    }

    #[test]
    fn missing_article_is_a_noindex_404() {
        SiteContent::set_global(fallback_content());

        // Слаг есть в адресе, но статьи с ним нет — это 404, и индексировать
        // его нельзя.
        let meta = page_meta("/blog/chernovik", SITE, None);
        assert!(meta.is_noindex());
        assert!(meta.title().contains("не найдена"));
    }

    #[test]
    fn every_page_has_an_og_image() {
        let meta = page_meta("/", SITE, None);
        let image = meta.image().expect("картинка для соцсетей");
        assert!(
            image.starts_with("https://inteli-dev.ru/og.png"),
            "og:image обязан быть абсолютным: {image}"
        );
    }

    #[test]
    fn description_is_clamped_by_whole_words() {
        let long = "слово ".repeat(60);
        let clamped = clamp_description(&long, 40);

        assert!(clamped.chars().count() <= 40);
        assert!(clamped.ends_with('…'));
        assert!(!clamped.contains("слов…"), "слово разрезано: {clamped}");

        // Короткий текст не трогаем.
        assert_eq!(clamp_description("  коротко  ", 40), "коротко");
    }

    #[test]
    fn clamped_description_survives_a_single_long_word() {
        let clamped = clamp_description(&"a".repeat(500), 20);
        assert!(clamped.chars().count() <= 20);
        assert!(clamped.ends_with('…'));
    }

    #[test]
    fn site_url_global_ignores_the_trailing_slash() {
        set_site_url("https://inteli-dev.ru/");
        assert_eq!(site_url(), "https://inteli-dev.ru");
        set_site_url("");
        assert_eq!(site_url(), "");
    }

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

        assert_eq!(
            metrika.tag_url(),
            "https://mc.yandex.ru/metrika/tag.js?id=113215363"
        );
        assert!(metrika.init_call().contains("ym(113215363, 'init'"));
        assert_eq!(metrika.watch_url(), "https://mc.yandex.ru/watch/113215363");
    }

    #[test]
    fn metrika_script_keeps_the_options_from_the_snippet() {
        let init = Metrika::new("113215363").expect("номер счётчика").init_call();

        // Опции из сниппета Метрики: без них счётчик собирает не то, что нужно.
        for option in [
            "ssr:true",
            "webvisor:true",
            "clickmap:true",
            "ecommerce:\"dataLayer\"",
            "accurateTrackBounce:true",
            "trackLinks:true",
        ] {
            assert!(init.contains(option), "в скрипте нет опции {option}");
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
    fn loader_is_absent_without_counters() {
        assert!(Counters::new(None, None).loader_script().is_none());
    }

    /// Внешние теги не должны попадать в критический путь загрузки: их
    /// подключает сам загрузчик после `load`, а не разметка.
    #[test]
    fn loader_defers_the_external_tags() {
        let script = Counters::new(Some("113215363"), Some("G-YNP4E8TF80"))
            .loader_script()
            .expect("загрузчик");

        assert!(
            script.contains("addEventListener('load'"),
            "счётчики грузятся до загрузки страницы"
        );
        assert!(
            script.contains("requestIdleCallback"),
            "нет загрузки в простое браузера"
        );
        assert!(
            script.contains("document.createElement('script')"),
            "внешние теги подключаются не скриптом, а разметкой"
        );
        // Очереди создаются сразу: иначе ранняя цель потеряется.
        assert!(script.contains("window.ym.a = window.ym.a || []"));
        assert!(script.contains("window.dataLayer = window.dataLayer || []"));
    }

    /// Синтетические прогоны (Lighthouse, PageSpeed Insights, WebPageTest) не
    /// должны получать счётчики: измерение не должно влиять на измеряемое.
    #[test]
    fn loader_skips_synthetic_runs() {
        let script = Counters::new(Some("113215363"), Some("G-YNP4E8TF80"))
            .loader_script()
            .expect("загрузчик");

        assert!(script.contains("navigator.webdriver"), "нет проверки автоматизации");
        for agent in ["Chrome-Lighthouse", "HeadlessChrome", "PTST", "GTmetrix"] {
            assert!(script.contains(agent), "нет проверки {agent}");
        }
    }

    /// Загрузчик собирается только из включённых счётчиков: выключенная
    /// Метрика не должна тянуть `mc.yandex.ru` в разметку.
    #[test]
    fn loader_contains_only_enabled_counters() {
        let metrika_only = Counters::new(Some("113215363"), None)
            .loader_script()
            .expect("загрузчик");
        assert!(metrika_only.contains("mc.yandex.ru"));
        assert!(!metrika_only.contains("googletagmanager"));
        assert!(!metrika_only.contains("gtag("));

        let google_only = Counters::new(None, Some("G-YNP4E8TF80"))
            .loader_script()
            .expect("загрузчик");
        assert!(google_only.contains("googletagmanager"));
        assert!(!google_only.contains("mc.yandex.ru"));
    }

    #[test]
    fn google_tag_scripts_carry_the_identifier() {
        let tag = GoogleTag::new("G-YNP4E8TF80").expect("идентификатор тега");

        assert_eq!(
            tag.script_src(),
            "https://www.googletagmanager.com/gtag/js?id=G-YNP4E8TF80"
        );

        let calls = tag.config_calls();
        assert!(calls.contains("gtag('js', new Date());"));
        assert!(calls.contains("gtag('config', 'G-YNP4E8TF80');"));
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

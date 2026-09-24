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
//!
//! Политика по ИИ-краулерам (кого пускать, кого нет) сознательно **не** задана:
//! это отдельное решение владельца, а не побочный эффект правки robots.txt.

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

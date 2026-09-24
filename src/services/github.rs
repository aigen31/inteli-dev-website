//! Статистика GitHub для блока «Открытый код» на главной странице.
//!
//! # Зачем отдельный слой
//!
//! Страницы рендерятся на сервере (SSR, без WASM), поэтому числа должны быть
//! готовы к моменту рендера — тянуть GitHub из компонента нельзя, это добавило
//! бы сетевую задержку в ответ на каждый запрос. Вместо этого сервис один раз
//! при старте и далее раз в `cache_ttl_seconds` обновляет снимок и публикует
//! его глобально ([`GitHubStats::get`]) — так же, как это делают `SiteContent`
//! и `Limits`. Если статистики нет (сеть недоступна, логин выключен), блок на
//! странице просто не рендерится.
//!
//! # Откуда данные
//!
//! * Публичный REST API (`/users/{u}`, `/users/{u}/repos`) — логин, аватар,
//!   число репозиториев, звёзды, подписчики, языки, дата регистрации.
//!   Работает анонимно.
//! * Контрибуции — единственная метрика, которой нет в REST. Берём её из
//!   GraphQL, если задан токен (точный JSON), иначе разбираем публичную
//!   страницу `/users/{u}/contributions`. Второй путь нужен, чтобы блок
//!   работал «из коробки»; если GitHub поменяет разметку, число просто
//!   перестанет показываться, а не сломает страницу.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::GitHubConfig;
use crate::error::{AppError, AppResult};

/// Сколько языков показываем отдельными сегментами, остальные — «Другие».
const TOP_LANGUAGES: usize = 6;
/// Таймаут одного запроса к GitHub.
const HTTP_TIMEOUT: Duration = Duration::from_secs(8);
/// Пауза перед повтором после неудачного обновления.
const RETRY_BACKOFF: Duration = Duration::from_secs(300);

/// Доля одного языка в проектах.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LanguageShare {
    pub name: String,
    pub repos: usize,
    /// Процент от репозиториев, у которых вообще определён язык.
    pub percent: u8,
    /// Цвет из палитры GitHub Linguist — чтобы полоса выглядела привычно.
    pub color: String,
}

/// Снимок статистики GitHub (то, что рисует UI).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GitHubStats {
    pub login: String,
    pub name: String,
    pub avatar_url: String,
    pub html_url: String,
    pub public_repos: u32,
    pub stars: u32,
    pub followers: u32,
    /// Контрибуции за последний год. `None` — не удалось получить.
    pub contributions_last_year: Option<u32>,
    pub years_on_github: u32,
    pub languages: Vec<LanguageShare>,
    /// Показывать ли звёзды и подписчиков (флаг из конфига). Кладём его в
    /// снимок, чтобы UI читал один глобальный источник, а не второй.
    pub show_stars: bool,
}

static STATS: RwLock<Option<GitHubStats>> = RwLock::new(None);

impl GitHubStats {
    /// Публикует свежий снимок. Ранее опубликованный снимок заменяется.
    pub fn set_global(stats: GitHubStats) {
        // Блокировка отравляется только при панике внутри `set_global`, а это
        // одна операция присваивания — восстанавливаемся вместо паники в UI.
        let mut guard = STATS.write().unwrap_or_else(|e| e.into_inner());
        *guard = Some(stats);
    }

    /// Текущий снимок. `None` — статистика ещё не загружена или выключена.
    pub fn get() -> Option<GitHubStats> {
        let guard = STATS.read().unwrap_or_else(|e| e.into_inner());
        guard.clone()
    }

    /// Есть ли что показывать: хотя бы аватар с числом репозиториев.
    pub fn is_meaningful(&self) -> bool {
        self.public_repos > 0
    }
}

/// Клиент GitHub с кэшем снимка.
#[derive(Debug, Clone)]
pub struct GitHubService {
    http: reqwest::Client,
    username: String,
    token: String,
    enabled: bool,
    show_stars: bool,
    ttl: Duration,
}

impl GitHubService {
    pub fn new(config: &GitHubConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .user_agent("inteli-dev.ru/1.0 (+https://inteli-dev.ru)")
            .build()
            .expect("reqwest client build cannot fail with valid config");

        Self {
            http,
            username: config.username.trim().trim_start_matches('@').to_string(),
            token: config.token.clone(),
            enabled: config.enabled && !config.username.trim().is_empty(),
            show_stars: config.show_stars,
            ttl: Duration::from_secs(config.cache_ttl_seconds.max(60)),
        }
    }

    /// Включён ли блок (непустой логин + флаг в конфиге).
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Запускает фоновое обновление снимка.
    ///
    /// Первый запрос выполняется сразу, поэтому блок появляется вскоре после
    /// старта; дальше — раз в TTL. Неудача не выключает блок навсегда: пробуем
    /// снова через [`RETRY_BACKOFF`], иначе разовый сетевой сбой при деплое
    /// оставил бы страницу без статистики до следующего перезапуска.
    pub fn spawn_refresh(self: Arc<Self>) {
        if !self.enabled {
            tracing::debug!("github: статистика выключена конфигом");
            return;
        }

        tokio::spawn(async move {
            loop {
                match self.fetch().await {
                    Ok(stats) => {
                        tracing::info!(
                            "github: {} — {} репозиториев, {} контрибуций",
                            stats.login,
                            stats.public_repos,
                            stats
                                .contributions_last_year
                                .map(|c| c.to_string())
                                .unwrap_or_else(|| "?".into())
                        );
                        GitHubStats::set_global(stats);
                        tokio::time::sleep(self.ttl).await;
                    }
                    Err(e) => {
                        tracing::warn!("github: не удалось обновить статистику: {e}");
                        tokio::time::sleep(RETRY_BACKOFF).await;
                    }
                }
            }
        });
    }

    /// Забирает статистику с GitHub (без кэша — кэш живёт в снимке).
    pub async fn fetch(&self) -> AppResult<GitHubStats> {
        let base = format!("https://api.github.com/users/{}", self.username);

        let user: RestUser = self.get_json(&base).await?;
        let repos: Vec<RestRepo> = self
            .get_json(&format!("{base}/repos?per_page=100&sort=pushed"))
            .await?;

        let stars = repos.iter().map(|r| r.stargazers_count).sum();
        let languages = top_languages(&repos);
        let contributions_last_year = self.fetch_contributions().await;

        Ok(GitHubStats {
            login: user.login,
            name: user.name.unwrap_or_default(),
            avatar_url: user.avatar_url,
            html_url: user.html_url,
            public_repos: user.public_repos,
            stars,
            followers: user.followers,
            contributions_last_year,
            years_on_github: years_since(&user.created_at).unwrap_or(0),
            languages,
            show_stars: self.show_stars,
        })
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> AppResult<T> {
        let mut req = self.http.get(url).header("Accept", "application/vnd.github+json");
        if !self.token.is_empty() {
            req = req.bearer_auth(&self.token);
        }

        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "github api {url}: HTTP {status}: {}",
                body.chars().take(200).collect::<String>()
            )));
        }
        Ok(resp.json::<T>().await?)
    }

    /// Контрибуции за год. Сначала GraphQL (если есть токен), затем — публичная
    /// страница. Любая ошибка здесь не фатальна: это одна метрика из четырёх.
    async fn fetch_contributions(&self) -> Option<u32> {
        if !self.token.is_empty() {
            match self.contributions_graphql().await {
                Ok(n) => return Some(n),
                Err(e) => tracing::debug!("github: graphql недоступен ({e}), пробуем разбор HTML"),
            }
        }
        match self.contributions_html().await {
            Ok(n) => Some(n),
            Err(e) => {
                tracing::warn!("github: не удалось получить контрибуции: {e}");
                None
            }
        }
    }

    async fn contributions_graphql(&self) -> AppResult<u32> {
        let query = "query($login:String!){user(login:$login){contributionsCollection\
                     {contributionCalendar{totalContributions}}}}";

        let resp = self
            .http
            .post("https://api.github.com/graphql")
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "query": query, "variables": { "login": self.username } }))
            .send()
            .await?;

        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            return Err(AppError::Internal(format!("graphql HTTP {status}")));
        }

        body["data"]["user"]["contributionsCollection"]["contributionCalendar"]
            ["totalContributions"]
            .as_u64()
            .map(|n| n as u32)
            .ok_or_else(|| AppError::Internal("graphql: нет totalContributions".into()))
    }

    /// Разбор публичной страницы `github.com/users/{u}/contributions`.
    async fn contributions_html(&self) -> AppResult<u32> {
        let url = format!("https://github.com/users/{}/contributions", self.username);
        let resp = self
            .http
            .get(&url)
            .header("Accept", "text/html")
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(AppError::Internal(format!("HTTP {}", resp.status())));
        }
        let html = resp.text().await?;
        parse_contributions_total(&html)
            .ok_or_else(|| AppError::Internal("не найдено число контрибуций в разметке".into()))
    }
}

/// REST-ответ `/users/{u}` — только нужные поля.
#[derive(Debug, Deserialize)]
struct RestUser {
    login: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    avatar_url: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    public_repos: u32,
    #[serde(default)]
    followers: u32,
    #[serde(default)]
    created_at: String,
}

/// REST-ответ `/users/{u}/repos` — только нужные поля.
#[derive(Debug, Deserialize)]
struct RestRepo {
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    stargazers_count: u32,
}

/// Считает распределение языков по репозиториям.
///
/// GitHub отдаёт `language = null` для репозиториев без кода (заметки, конфиги),
/// поэтому процент берём от репозиториев *с* языком — иначе полоса никогда не
/// заполнялась бы целиком.
fn top_languages(repos: &[RestRepo]) -> Vec<LanguageShare> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for repo in repos {
        let Some(lang) = repo.language.as_deref().filter(|l| !l.is_empty()) else {
            continue;
        };
        match counts.iter_mut().find(|(name, _)| name == lang) {
            Some((_, n)) => *n += 1,
            None => counts.push((lang.to_string(), 1)),
        }
    }

    let total: usize = counts.iter().map(|(_, n)| n).sum();
    if total == 0 {
        return Vec::new();
    }

    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let mut shares: Vec<LanguageShare> = counts
        .iter()
        .take(TOP_LANGUAGES)
        .map(|(name, n)| LanguageShare {
            name: name.clone(),
            repos: *n,
            percent: ((*n as f64 / total as f64) * 100.0).round() as u8,
            color: language_color(name).to_string(),
        })
        .collect();

    let rest: usize = counts.iter().skip(TOP_LANGUAGES).map(|(_, n)| n).sum();
    if rest > 0 {
        shares.push(LanguageShare {
            name: "Другие".to_string(),
            repos: rest,
            percent: ((rest as f64 / total as f64) * 100.0).round() as u8,
            color: "#6b7280".to_string(),
        });
    }

    shares
}

/// Вытаскивает общее число контрибуций из HTML страницы `/users/{u}/contributions`.
///
/// Настоящая разметка многострочная и с отступами:
///
/// ```html
/// <h2 id="js-contribution-activity-description" class="f4 text-normal mb-2">
///   125
///   contributions
///     in the last year
/// </h2>
/// ```
///
/// Поэтому парсер не рассчитывает на одиночные пробелы: привязываемся к хвосту
/// `in the last year`, отрезаем слово `contribution(s)` вместе с окружающими
/// пробелами и дочитываем назад цифры. Так учитываются и переносы строк, и
/// разделитель тысяч (`1,234`), и единственное число (`1 contribution`).
///
/// Перебираем все вхождения хвоста, а не только первое: та же фраза может
/// встретиться раньше в служебной разметке страницы.
fn parse_contributions_total(html: &str) -> Option<u32> {
    const TAIL: &str = "in the last year";

    for (idx, _) in html.match_indices(TAIL) {
        let head = html[..idx].trim_end();
        let Some(head) = head
            .strip_suffix("contributions")
            .or_else(|| head.strip_suffix("contribution"))
        else {
            continue;
        };

        let digits: String = head
            .trim_end()
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit() || *c == ',')
            .filter(|c| c.is_ascii_digit())
            .collect::<String>()
            .chars()
            .rev()
            .collect();

        if let Ok(n) = digits.parse::<u32>() {
            return Some(n);
        }
    }
    None
}

/// Сколько полных лет прошло с даты регистрации (RFC3339).
fn years_since(created_at: &str) -> Option<u32> {
    let created: DateTime<Utc> = created_at.parse().ok()?;
    let days = (Utc::now() - created).num_days();
    (days >= 0).then(|| (days / 365) as u32)
}

/// Цвета GitHub Linguist для популярных языков; остальные — нейтральный.
fn language_color(name: &str) -> &'static str {
    match name {
        "JavaScript" => "#f1e05a",
        "TypeScript" => "#3178c6",
        "PHP" => "#4F5D95",
        "HTML" => "#e34c26",
        "CSS" => "#563d7c",
        "SCSS" => "#c6538c",
        "Python" => "#3572A5",
        "Rust" => "#dea584",
        "Go" => "#00ADD8",
        "Java" => "#b07219",
        "Kotlin" => "#A97BFF",
        "C" => "#555555",
        "C++" => "#f34b7d",
        "C#" => "#178600",
        "Ruby" => "#701516",
        "Shell" => "#89e051",
        "Dockerfile" => "#384d54",
        "Vue" => "#41b883",
        "Svelte" => "#ff3e00",
        "Blade" => "#f7523f",
        "Lua" => "#000080",
        "Nix" => "#7e7eff",
        "HCL" => "#844FBA",
        "Makefile" => "#427819",
        "Jupyter Notebook" => "#DA5B0B",
        _ => "#8b8b9e",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(lang: Option<&str>, stars: u32) -> RestRepo {
        RestRepo {
            language: lang.map(|s| s.to_string()),
            stargazers_count: stars,
        }
    }

    #[test]
    fn languages_are_ranked_and_sum_to_100() {
        let repos = vec![
            repo(Some("JavaScript"), 0),
            repo(Some("JavaScript"), 0),
            repo(Some("PHP"), 0),
            repo(None, 0),
        ];
        let shares = top_languages(&repos);

        assert_eq!(shares.len(), 2);
        assert_eq!(shares[0].name, "JavaScript");
        assert_eq!(shares[0].repos, 2);
        assert_eq!(shares[0].percent, 67);
        assert_eq!(shares[0].color, "#f1e05a");
        assert_eq!(shares[1].name, "PHP");
        // Проценты считаются от репозиториев с языком, поэтому сумма = 100.
        assert_eq!(shares.iter().map(|s| s.percent).sum::<u8>(), 100);
    }

    #[test]
    fn languages_overflow_into_other_bucket() {
        let repos = vec![
            repo(Some("Rust"), 0),
            repo(Some("Go"), 0),
            repo(Some("Python"), 0),
            repo(Some("Ruby"), 0),
            repo(Some("Java"), 0),
            repo(Some("Kotlin"), 0),
            repo(Some("Lua"), 0),
            repo(Some("Nix"), 0),
        ];
        let shares = top_languages(&repos);

        // 6 языков + «Другие».
        assert_eq!(shares.len(), TOP_LANGUAGES + 1);
        assert_eq!(shares.last().unwrap().name, "Другие");
        assert_eq!(shares.last().unwrap().repos, 2);
        assert_eq!(shares.iter().map(|s| s.repos).sum::<usize>(), 8);
    }

    #[test]
    fn no_languages_is_not_a_panic() {
        assert!(top_languages(&[repo(None, 3)]).is_empty());
    }

    #[test]
    fn parses_contributions_from_real_markup() {
        // Точная копия разметки GitHub (многострочный h2 с отступами) —
        // именно на ней сломался первый вариант парсера, рассчитанный на
        // «125 contributions in the last year» в одну строку.
        let html = r#"<div class="js-yearly-contributions">
  <div class="position-relative">
    <h2 tabindex="-1" id="js-contribution-activity-description" class="f4 text-normal mb-2">
      125
      contributions
        in the last year
    </h2>
    <td data-date="2026-01-01" data-level="2">1 contribution</td>
  </div>
</div>"#;
        assert_eq!(parse_contributions_total(html), Some(125));
    }

    #[test]
    fn parses_thousands_separator_and_whitespace() {
        let html = "<h2>\n      1,234\n      contributions\n      in the last year\n    </h2>";
        assert_eq!(parse_contributions_total(html), Some(1234));
    }

    #[test]
    fn parses_single_contribution() {
        // Единственное число тоже должно разбираться.
        assert_eq!(
            parse_contributions_total("<h2>1 contribution in the last year</h2>"),
            Some(1)
        );
    }

    #[test]
    fn first_usable_occurrence_wins() {
        // Раньше по документу та же фраза встречается в служебном тексте —
        // парсер обязан пропустить её и дойти до настоящего заголовка.
        let html = r#"<meta content="contribution activity in the last year">
            <h2>42 contributions in the last year</h2>"#;
        assert_eq!(parse_contributions_total(html), Some(42));
    }

    #[test]
    fn missing_contributions_marker_returns_none() {
        assert_eq!(parse_contributions_total("<html>no stats</html>"), None);
    }

    #[test]
    fn years_on_github_uses_full_years() {
        let ten_years_ago = (Utc::now() - chrono::Duration::days(365 * 10 + 5))
            .to_rfc3339();
        assert_eq!(years_since(&ten_years_ago), Some(10));

        let five_months = (Utc::now() - chrono::Duration::days(150)).to_rfc3339();
        assert_eq!(years_since(&five_months), Some(0));

        assert_eq!(years_since("not a date"), None);
    }

    #[test]
    fn language_color_is_stable_for_unknown() {
        assert_eq!(language_color("Brainfuck"), "#8b8b9e");
        assert_ne!(language_color("Rust"), language_color("PHP"));
    }
}

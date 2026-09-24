//! Блок «Открытый код» на главной: статистика GitHub.
//!
//! Данные приходят из глобального [`GitHubStats`], который наполняет фоновая
//! задача [`crate::services::github::GitHubService::spawn_refresh`]. Если снимка
//! нет — компонент не рендерит ничего: пустая рамка с нулями хуже, чем её
//! отсутствие.

use leptos::prelude::*;
use leptos::svg;

use crate::limits::plural_ru;
use crate::services::github::GitHubStats;

/// Логотип GitHub (octocat mark).
///
/// Это brand-mark, а не иконка Lucide: Lucide принципиально не содержит
/// брендовых логотипов, поэтому `scripts/update-icons.sh` его не генерирует и
/// путь задаётся здесь. Заливка, а не обводка — как в оригинале.
#[component]
pub fn GitHubMark() -> impl IntoView {
    const MARK: &str = r#"<path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12" />"#;

    svg::svg()
        .attr("xmlns", "http://www.w3.org/2000/svg")
        .attr("viewBox", "0 0 24 24")
        .attr("fill", "currentColor")
        .attr("width", "1em")
        .attr("height", "1em")
        .attr("aria-hidden", "true")
        .inner_html(MARK)
}

/// Блок статистики GitHub. Ничего не рендерит, пока нет снимка.
#[component]
pub fn GitHubStatsSection() -> impl IntoView {
    section_view(GitHubStats::get().filter(GitHubStats::is_meaningful))
}

/// Решение «рендерить или нет» вынесено отдельно от чтения глобального
/// снимка, чтобы оба случая можно было проверить тестом.
fn section_view(stats: Option<GitHubStats>) -> AnyView {
    match stats {
        None => view! { <></> }.into_any(),
        Some(stats) => render_stats(stats).into_any(),
    }
}

/// Собирает плитки показателей. Вынесено из компонента, чтобы набор плиток
/// (включая флаг `show_stars`) был покрыт юнит-тестом.
fn build_tiles(stats: &GitHubStats) -> Vec<(String, String)> {
    let mut tiles: Vec<(String, String)> = Vec::new();

    if let Some(contributions) = stats.contributions_last_year {
        tiles.push((
            contributions.to_string(),
            plural_ru(
                contributions,
                "вклад за год",
                "вклада за год",
                "вкладов за год",
            )
            .to_string(),
        ));
    }

    tiles.push((
        stats.public_repos.to_string(),
        plural_ru(
            stats.public_repos,
            "публичный репозиторий",
            "публичных репозитория",
            "публичных репозиториев",
        )
        .to_string(),
    ));

    if !stats.languages.is_empty() {
        let n = stats.languages.len() as u32;
        tiles.push((
            n.to_string(),
            plural_ru(n, "язык в проектах", "языка в проектах", "языков в проектах").to_string(),
        ));
    }

    if stats.years_on_github > 0 {
        tiles.push((
            stats.years_on_github.to_string(),
            plural_ru(
                stats.years_on_github,
                "год на GitHub",
                "года на GitHub",
                "лет на GitHub",
            )
            .to_string(),
        ));
    }

    // Звёзды и подписчики — только по флагу из конфига: маленькие числа
    // («2 звезды», «1 подписчик») на странице продаж работают против нас.
    if stats.show_stars {
        tiles.push((
            stats.stars.to_string(),
            plural_ru(stats.stars, "звезда", "звезды", "звёзд").to_string(),
        ));
        tiles.push((
            stats.followers.to_string(),
            plural_ru(stats.followers, "подписчик", "подписчика", "подписчиков").to_string(),
        ));
    }

    tiles
}

fn render_stats(stats: GitHubStats) -> impl IntoView {
    let login = stats.login.clone();
    let profile_url = stats.html_url.clone();
    let profile_label = stats
        .html_url
        .trim_start_matches("https://")
        .trim_end_matches('/')
        .to_string();
    let avatar = stats.avatar_url.clone();
    let display_name = if stats.name.is_empty() {
        login.clone()
    } else {
        stats.name.clone()
    };
    let tiles = build_tiles(&stats);
    let languages = stats.languages.clone();

    view! {
        <section class="section section-glow section-github">
            <div class="section-head">
                <h2>{"Открытый код"}</h2>
                <a
                    class="section-link"
                    href=profile_url
                    target="_blank"
                    rel="noopener noreferrer nofollow"
                >
                    {format!("{profile_label} →")}
                </a>
            </div>

            <div class="github-panel">
                <a
                    class="github-profile"
                    href=stats.html_url.clone()
                    target="_blank"
                    rel="noopener noreferrer nofollow"
                >
                    <img
                        class="github-avatar"
                        src=avatar
                        alt=""
                        width="56"
                        height="56"
                        loading="lazy"
                        decoding="async"
                    />
                    <span class="github-profile-text">
                        <span class="github-profile-name">{display_name}</span>
                        <span class="github-profile-handle">
                            <GitHubMark/>
                            {format!("@{login}")}
                        </span>
                    </span>
                </a>

                <div class="github-stats">
                    {tiles
                        .into_iter()
                        .map(|(value, label)| {
                            view! {
                                <div class="github-stat">
                                    <span class="github-stat-value">{value}</span>
                                    <span class="github-stat-label">{label}</span>
                                </div>
                            }
                        })
                        .collect::<Vec<_>>()}
                </div>

                {(!languages.is_empty())
                    .then(|| {
                        view! {
                            <div class="github-langs">
                                <div
                                    class="github-lang-bar"
                                    role="img"
                                    aria-label="Распределение языков по публичным репозиториям"
                                >
                                    {languages
                                        .iter()
                                        .map(|l| {
                                            view! {
                                                <span
                                                    class="github-lang-seg"
                                                    style=format!(
                                                        "width:{}%;background:{}",
                                                        l.percent,
                                                        l.color,
                                                    )
                                                    title=format!("{} — {} реп.", l.name, l.repos)
                                                ></span>
                                            }
                                        })
                                        .collect::<Vec<_>>()}
                                </div>
                                <ul class="github-lang-legend">
                                    {languages
                                        .iter()
                                        .map(|l| {
                                            view! {
                                                <li class="github-lang-item">
                                                    <span
                                                        class="github-lang-dot"
                                                        style=format!("background:{}", l.color)
                                                        aria-hidden="true"
                                                    ></span>
                                                    <span class="github-lang-name">
                                                        {l.name.clone()}
                                                    </span>
                                                    <span class="github-lang-count">
                                                        {format!("{} реп.", l.repos)}
                                                    </span>
                                                </li>
                                            }
                                        })
                                        .collect::<Vec<_>>()}
                                </ul>
                            </div>
                        }
                    })}
            </div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::github::LanguageShare;

    fn sample(show_stars: bool) -> GitHubStats {
        GitHubStats {
            login: "aigen31".into(),
            name: "AiGen".into(),
            avatar_url: "https://avatars.githubusercontent.com/u/22431602?v=4".into(),
            html_url: "https://github.com/aigen31".into(),
            public_repos: 48,
            stars: 2,
            followers: 1,
            contributions_last_year: Some(125),
            years_on_github: 10,
            languages: vec![LanguageShare {
                name: "JavaScript".into(),
                repos: 9,
                percent: 33,
                color: "#f1e05a".into(),
            }],
            show_stars,
        }
    }

    #[test]
    fn tiles_use_correct_russian_plurals() {
        let tiles = build_tiles(&sample(false));
        let labels: Vec<&str> = tiles.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(
            labels,
            vec![
                "вкладов за год",
                "публичных репозиториев",
                "язык в проектах",
                "лет на GitHub",
            ]
        );
        assert_eq!(tiles[0].0, "125");
        assert_eq!(tiles[1].0, "48");
    }

    #[test]
    fn stars_and_followers_are_hidden_by_default() {
        let hidden = build_tiles(&sample(false));
        assert_eq!(hidden.len(), 4);
        assert!(!hidden.iter().any(|(_, l)| l.contains("звезд") || l.contains("подписчик")));

        let shown = build_tiles(&sample(true));
        assert_eq!(shown.len(), 6);
        assert_eq!(shown[4].1, "звезды");
        assert_eq!(shown[5].1, "подписчик");
    }

    #[test]
    fn tiles_without_contributions_still_work() {
        let mut stats = sample(false);
        stats.contributions_last_year = None;
        stats.years_on_github = 0;
        let labels: Vec<String> = build_tiles(&stats)
            .into_iter()
            .map(|(_, l)| l)
            .collect();
        assert_eq!(labels, vec!["публичных репозиториев", "язык в проектах"]);
    }

    #[test]
    fn snapshot_round_trips_through_global() {
        GitHubStats::set_global(sample(false));
        let got = GitHubStats::get().expect("снимок опубликован");
        assert_eq!(got.login, "aigen31");
        assert_eq!(got.contributions_last_year, Some(125));
        assert!(!got.show_stars);
        assert!(got.is_meaningful());
    }

    #[test]
    fn meaningless_snapshot_is_not_rendered() {
        let mut empty = sample(false);
        empty.public_repos = 0;
        assert!(!empty.is_meaningful());
    }

    #[test]
    fn missing_snapshot_renders_nothing_at_all() {
        let html = section_view(None).to_html();
        assert!(
            !html.contains("Открытый код") && !html.contains("github-panel"),
            "без снимка секции быть не должно, получено: {html}"
        );
    }

    #[test]
    fn rendered_section_contains_stats_and_languages() {
        let html = section_view(Some(sample(false))).to_html();

        assert!(html.contains("Открытый код"));
        assert!(html.contains("github.com/aigen31"));
        assert!(html.contains("@aigen31"));
        assert!(html.contains("вкладов за год"));
        assert!(html.contains("публичных репозиториев"));
        // Аватар отдаётся с CDN GitHub с фиксированными размерами (нет CLS).
        assert!(html.contains("avatars.githubusercontent.com"));
        assert!(html.contains(r#"width="56""#));
        // Легенда языков и сегмент полосы с цветом Linguist.
        assert!(html.contains("JavaScript"));
        assert!(html.contains("#f1e05a"));
        // Звёзды/подписчики выключены — их в разметке нет.
        assert!(!html.contains("звезд"));
        assert!(!html.contains("подписчик"));
        // Внешние ссылки обязаны быть noopener (иначе tabnabbing).
        assert!(html.contains(r#"rel="noopener noreferrer nofollow""#));
    }
}

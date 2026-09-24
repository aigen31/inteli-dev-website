//! Публичные страницы блога: список `/blog` и статья `/blog/{slug}`.
//!
//! Страницы рендерятся синхронно из [`PublishedArticles`] — снимка
//! опубликованных статей в памяти процесса, который пересобирается после
//! каждой правки в админке. Черновики и архив в снимок не попадают, поэтому
//! «случайно опубликовать черновик» через блог невозможно.

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::services::article::{Article, ArticleHead, PublishedArticles};
use crate::ui::NotFound;

/// Сколько последних статей показывать тизером на главной.
pub const HOME_TEASER_COUNT: usize = 3;

/// Форматирует RFC3339-дату как «24 сентября 2026».
///
/// Без внешнего форматтера: нужен один формат на весь сайт, а `chrono`
/// локали тянет за собой лишний код.
pub fn format_date_ru(rfc3339: &str) -> String {
    use chrono::Datelike;

    const MONTHS: [&str; 12] = [
        "января", "февраля", "марта", "апреля", "мая", "июня",
        "июля", "августа", "сентября", "октября", "ноября", "декабря",
    ];

    match chrono::DateTime::parse_from_rfc3339(rfc3339) {
        Ok(dt) => format!(
            "{} {} {}",
            dt.day(),
            MONTHS[(dt.month0() as usize).min(11)],
            dt.year()
        ),
        // Битый таймстемп не должен ломать страницу: показываем дату как есть.
        Err(_) => rfc3339.chars().take(10).collect(),
    }
}

/// Тег статьи. Обычная функция, а не компонент: пропсов-реактивности здесь нет.
fn tag_list(tags: &[String]) -> AnyView {
    if tags.is_empty() {
        return ().into_any();
    }
    view! {
        <ul class="tag-list">
            {tags
                .iter()
                .cloned()
                .map(|tag| view! { <li class="tag">{tag}</li> })
                .collect_view()}
        </ul>
    }
    .into_any()
}

/// Карточка статьи в списке.
fn article_card(article: ArticleHead) -> AnyView {
    let href = article.url_path();
    let date = article
        .published_at
        .as_deref()
        .map(format_date_ru)
        .unwrap_or_default();
    let reading = article.reading_time_minutes;
    let cover = article.cover_image_url.clone();
    let summary = if article.summary.trim().is_empty() {
        None
    } else {
        Some(article.summary.clone())
    };
    let tags = tag_list(&article.tags);

    view! {
        <article class="post-card">
            {cover.map(|src| view! {
                <a class="post-card-cover" href=href.clone() tabindex="-1" aria-hidden="true">
                    <img src=src alt="" loading="lazy"/>
                </a>
            })}
            <div class="post-card-body">
                <h2 class="post-card-title"><a href=href.clone()>{article.title.clone()}</a></h2>
                <p class="post-meta">
                    <time datetime=article.published_at.clone().unwrap_or_default()>{date}</time>
                    <span aria-hidden="true">{"·"}</span>
                    <span>{reading} " мин чтения"</span>
                </p>
                {summary.map(|s| view! { <p class="post-card-summary">{s}</p> })}
                {tags}
            </div>
        </article>
    }
    .into_any()
}

/// Страница со списком статей.
#[component]
pub fn Blog() -> impl IntoView {
    let articles = PublishedArticles::heads();

    view! {
        <section class="section blog-section">
            <header class="blog-header">
                <h1>Блог</h1>
                <p class="subtitle">
                    "Заметки о приватных AI-системах, локальном инференсе и разработке."
                </p>
            </header>

            {if articles.is_empty() {
                view! {
                    <p class="empty-state">
                        "Статьи ещё не опубликованы. Загляните позже."
                    </p>
                }.into_any()
            } else {
                view! {
                    <div class="post-list">
                        {articles.into_iter().map(article_card).collect_view()}
                    </div>
                }.into_any()
            }}
        </section>
    }
}

/// Страница статьи.
#[component]
pub fn BlogPost() -> impl IntoView {
    let params = use_params_map();
    // SSR рендерит страницу один раз, поэтому параметр читаем синхронно.
    let slug = params.read().get("slug").unwrap_or_default();
    let article = PublishedArticles::find(&slug);

    let Some(article) = article else {
        // Правильный 404, а не «мягкая» страница с кодом 200: иначе поисковики
        // проиндексируют несуществующие адреса.
        if let Some(options) = use_context::<leptos_axum::ResponseOptions>() {
            options.set_status(axum::http::StatusCode::NOT_FOUND);
        }
        return view! { <NotFound/> }.into_any();
    };

    let date = article
        .published_at
        .as_deref()
        .map(format_date_ru)
        .unwrap_or_default();
    let reading = article.reading_time_minutes();
    // HTML собрал наш рендерер: сырой HTML экранирован, ссылки с опасными
    // схемами вырезаны (см. `utils::markdown`).
    let body = article.html();

    view! {
        <article class="section post-section">
            <nav class="breadcrumbs" aria-label="Хлебные крошки">
                <a href="/blog">"Блог"</a>
                <span aria-hidden="true">{"→"}</span>
                <span>{article.title.clone()}</span>
            </nav>

            <header class="post-header">
                <h1>{article.title.clone()}</h1>
                <p class="post-meta">
                    <time datetime=article.published_at.clone().unwrap_or_default()>{date}</time>
                    <span aria-hidden="true">{"·"}</span>
                    <span>{reading} " мин чтения"</span>
                </p>
                {tag_list(&article.tags)}
            </header>            {article.cover_image_url.clone().map(|src| view! {
                <img class="post-cover" src=src alt="" loading="lazy"/>
            })}

            <div class="prose" inner_html=body></div>

            <footer class="post-footer">
                <p>
                    "Есть задача по этой теме? "
                    <a href="/contact">"Напишите мне"</a>
                    " — отвечу в течение дня."
                </p>
                <a class="btn btn-secondary" href="/blog">"← Все статьи"</a>
            </footer>
        </article>
    }
    .into_any()
}

/// Тизер последних статей для главной страницы.
///
/// Возвращает `None`, если статей нет: блок-пустышка на главной только мешает.
#[component]
pub fn LatestArticles() -> impl IntoView {
    let articles: Vec<Article> = PublishedArticles::list()
        .into_iter()
        .take(HOME_TEASER_COUNT)
        .collect();

    if articles.is_empty() {
        return ().into_any();
    }

    view! {
        <section class="section blog-teaser">
            <div class="section-head">
                <h2>Последние статьи</h2>
                <a class="section-more" href="/blog">"Все статьи →"</a>
            </div>
            <div class="post-list">
                {articles.into_iter().map(|a| article_card(a.head())).collect_view()}
            </div>
        </section>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_is_formatted_in_russian() {
        assert_eq!(format_date_ru("2026-09-24T12:36:13Z"), "24 сентября 2026");
        assert_eq!(format_date_ru("2026-01-01T00:00:00+03:00"), "1 января 2026");
        assert_eq!(format_date_ru("2026-12-31T23:59:59Z"), "31 декабря 2026");
    }

    #[test]
    fn broken_date_does_not_panic() {
        assert_eq!(format_date_ru("не дата"), "не дата");
        assert_eq!(format_date_ru(""), "");
    }
}

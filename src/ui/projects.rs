//! Страницы проектов: список `/projects` и отдельный кейс `/projects/{slug}`.
//!
//! Факты о кейсах живут в `memory::content::Project` (источник — портфолио
//! автора): карточка в списке и страница кейса читают одну структуру, поэтому
//! «кратко» и «подробно» не могут разойтись.

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::api::assets;
use crate::memory::content::{Project, SiteContent};
use crate::ui::shared::{ProjectCard, SecondaryButton};
use crate::ui::NotFound;

/// Страница со списком кейсов.
#[component]
pub fn Projects() -> impl IntoView {
    let content = SiteContent::get();
    let projects = content.projects.clone();

    view! {
        <section class="section projects-section">
            <header class="section-head">
                <p class="eyebrow"><span class="eyebrow-dot" aria-hidden="true"></span>{"Кейсы 2023—2026"}</p>
                <h1>Проекты</h1>
            </header>
            <p class="subtitle">
                "Задачи, которые я делал целиком или в команде: что требовалось, \
                 что сделано и что получилось. У каждого кейса — своя страница с деталями."
            </p>
            <div class="project-list">
                {projects.into_iter().map(|p| view! { <ProjectCard project=p level=2/> }).collect::<Vec<_>>()}
            </div>
            <div class="cta-buttons">
                <SecondaryButton text="Хочу так же" href="/contact"/>
            </div>
        </section>
    }
}

/// Страница одного кейса.
#[component]
pub fn ProjectPage() -> impl IntoView {
    let params = use_params_map();
    // SSR рендерит страницу один раз, поэтому параметр читаем синхронно.
    let slug = params.read().get("slug").unwrap_or_default();
    let project = SiteContent::get()
        .projects
        .iter()
        .find(|p| p.slug == slug)
        .cloned();

    let Some(project) = project else {
        // Настоящий 404, а не «мягкая» страница с кодом 200: иначе поисковики
        // проиндексируют несуществующие адреса.
        if let Some(options) = use_context::<leptos_axum::ResponseOptions>() {
            options.set_status(axum::http::StatusCode::NOT_FOUND);
        }
        return view! { <NotFound/> }.into_any();
    };

    let image_path = assets::project_image_path(&project.image);
    let image = assets::project_image(&project.image);
    let (img_w, img_h) = image.map(|i| (i.width, i.height)).unwrap_or((1600, 900));
    let site = project.site_url.clone();
    let client = project.client.clone();
    let stack = project.stack.clone();
    let solution = project.solution.clone();
    let result = project.result.clone();
    let (prev, next) = neighbours(&project.slug);
    // Ссылку и заголовок соседа считаем заранее: `view!` берёт `url_path()` и
    // `title` из одного значения, и порядок вычисления иначе упирается в
    // частичное перемещение.
    let prev_link = prev.map(|p| (p.url_path(), p.title));
    let next_link = next.map(|p| (p.url_path(), p.title));

    view! {
        <article class="section project-page">
            <nav class="breadcrumbs" aria-label="Хлебные крошки">
                <a href="/projects">"Проекты"</a>
                <span aria-hidden="true">"→"</span>
                <span>{project.title.clone()}</span>
            </nav>

            <header class="project-page-header">
                <p class="project-card-meta">
                    <span class="project-card-niche">{project.niche.clone()}</span>
                    {client.clone().map(|c| view! { <span class="project-card-client">{c}</span> })}
                </p>
                <h1>{project.title.clone()}</h1>
                <p class="project-page-lead">{project.summary.clone()}</p>
            </header>

            // Скриншот кейса — тот же файл, что в карточке. Клик открывает его
            // целиком: 1600 px в колонке 1120 px иначе не прочитать.
            <figure class="project-page-figure">
                <img
                    class="project-shot"
                    src=assets::versioned(&image_path)
                    alt=format!("Скриншот проекта «{}»", project.title)
                    width=img_w
                    height=img_h
                    decoding="async"
                />
            </figure>

            <div class="project-page-facts">
                <div class="project-fact">
                    <span class="project-fact-label">"Результат"</span>
                    <span class="project-fact-value">{project.outcome.clone()}</span>
                </div>
                {site.map(|url| {
                    let label = display_host(&url);
                    view! {
                        <div class="project-fact">
                            <span class="project-fact-label">"Сайт"</span>
                            <a class="project-fact-link" href=url target="_blank" rel="noopener noreferrer">
                                {label}
                                <span aria-hidden="true">"↗"</span>
                            </a>
                        </div>
                    }
                })}
            </div>

            <div class="project-page-body">
                <section class="project-block">
                    <h2>Задача</h2>
                    <p>{project.task.clone()}</p>
                </section>

                <section class="project-block">
                    <h2>Что сделано</h2>
                    <ul class="project-list-check">
                        {solution.into_iter().map(|item| view! { <li>{item}</li> }).collect::<Vec<_>>()}
                    </ul>
                </section>

                <section class="project-block">
                    <h2>Результат</h2>
                    <ul class="project-list-check project-list-result">
                        {result.into_iter().map(|item| view! { <li>{item}</li> }).collect::<Vec<_>>()}
                    </ul>
                </section>

                {(!stack.is_empty()).then(|| view! {
                    <section class="project-block">
                        <h2>Стек</h2>
                        <ul class="project-stack">
                            {stack.into_iter().map(|tech| view! { <li>{tech}</li> }).collect::<Vec<_>>()}
                        </ul>
                    </section>
                })}
            </div>

            <footer class="project-page-footer">
                <p class="project-page-cta">
                    "Похожая задача? "
                    <a href="/contact">"Напишите"</a>
                    " — отвечу при наличии свободного окна."
                </p>
            </footer>

            <nav class="project-nav" aria-label="Другие кейсы">
                {prev_link.map(|(url, title)| view! {
                    <a class="project-nav-link project-nav-prev" href=url>
                        <span class="project-nav-label" aria-hidden="true">"← Предыдущий"</span>
                        <span class="project-nav-title">{title}</span>
                    </a>
                })}
                {next_link.map(|(url, title)| view! {
                    <a class="project-nav-link project-nav-next" href=url>
                        <span class="project-nav-label" aria-hidden="true">"Следующий →"</span>
                        <span class="project-nav-title">{title}</span>
                    </a>
                })}
            </nav>
        </article>
    }
    .into_any()
}

/// Соседние кейсы по кругу: «предыдущий» и «следующий» на странице кейса.
fn neighbours(slug: &str) -> (Option<Project>, Option<Project>) {
    let projects = &SiteContent::get().projects;
    let Some(index) = projects.iter().position(|p| p.slug == slug) else {
        return (None, None);
    };
    let count = projects.len();
    if count < 2 {
        return (None, None);
    }
    let prev = projects[(index + count - 1) % count].clone();
    let next = projects[(index + 1) % count].clone();
    (Some(prev), Some(next))
}

/// Хост из адреса проекта — для подписи ссылки: `https://pdd.pro` → `pdd.pro`.
fn display_host(url: &str) -> String {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .trim_end_matches('/')
        .split('/')
        .next()
        .unwrap_or(url)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::content::fallback_content;

    /// Соседи по кругу: с последнего кейса «следующий» — первый, иначе на
    /// краю списка одна из ссылок пропадала бы.
    #[test]
    fn neighbours_wrap_around_the_list() {
        SiteContent::set_global(fallback_content());
        let all = SiteContent::get().projects.clone();
        assert!(all.len() > 2);

        let (prev, next) = neighbours(&all[0].slug);
        assert_eq!(
            prev.map(|p| p.slug),
            Some(all.last().expect("последний кейс").slug.clone())
        );
        assert_eq!(next.map(|p| p.slug), Some(all[1].slug.clone()));

        let (_, next_last) = neighbours(&all.last().expect("последний кейс").slug);
        assert_eq!(next_last.map(|p| p.slug), Some(all[0].slug.clone()));

        let (none_prev, none_next) = neighbours("no-such-case");
        assert!(none_prev.is_none() && none_next.is_none());
    }

    /// В подписи ссылки — только хост: полный адрес с путём читается как код.
    #[test]
    fn host_is_shortened_for_the_link_label() {
        assert_eq!(display_host("https://pdd.pro"), "pdd.pro");
        assert_eq!(display_host("https://ormas.pro/"), "ormas.pro");
        assert_eq!(display_host("https://x.ru/a/b"), "x.ru");
    }
}

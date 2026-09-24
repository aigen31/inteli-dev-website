//! Рендер markdown → HTML для статей блога.
//!
//! Один рендерер на всё приложение: страница статьи, RSS-описание и
//! предпросмотр в админке используют его же, поэтому предпросмотр не может
//! разойтись с тем, что увидит читатель.
//!
//! # Безопасность
//!
//! Текст статьи пишет владелец сайта, но это не повод пропускать сырой HTML:
//! тот же рендерер будет обрабатывать статьи, пришедшие из n8n. Поэтому
//! [`render`] экранирует `Html`/`InlineHtml` (они становятся текстом) и
//! вычищает опасные схемы ссылок (`javascript:`, `data:` и т.п.).

use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag};

/// Возможности markdown, которые мы включаём в статьях.
///
/// Список намеренно фиксирован и не читается из контента: набор возможностей
/// не должен зависеть от данных.
fn options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    options
}

/// Рендерит markdown в готовый HTML.
pub fn render(markdown: &str) -> String {
    let parser = Parser::new_ext(markdown, options()).map(sanitize_event);
    // Небольшой запас: HTML обычно чуть длиннее исходного markdown.
    let mut out = String::with_capacity(markdown.len() + markdown.len() / 4);
    html::push_html(&mut out, parser);
    out
}

/// Плоский текст статьи без разметки — для описания в RSS и оценки объёма.
pub fn to_plain_text(markdown: &str) -> String {
    let mut raw = String::with_capacity(markdown.len());
    for event in Parser::new_ext(markdown, options()) {
        match event {
            Event::Text(text) | Event::Code(text) => raw.push_str(&text),
            // Любой разрыв — пробел; лишние пробелы схлопнем в конце. Так не
            // нужно угадывать конкретные варианты `TagEnd`, которые менялись
            // между версиями pulldown-cmark.
            Event::SoftBreak | Event::HardBreak | Event::End(_) => raw.push(' '),
            _ => {}
        }
    }
    collapse_whitespace(&raw)
}

/// Оценка времени чтения в минутах (минимум одна).
///
/// 180 слов в минуту — средний темп чтения про себя для русского текста.
pub fn reading_time_minutes(markdown: &str) -> u32 {
    const WORDS_PER_MINUTE: usize = 180;
    let words = to_plain_text(markdown).split_whitespace().count();
    (words.div_ceil(WORDS_PER_MINUTE).max(1)) as u32
}

/// Обрезает плоский текст до `max_chars` символов по границе слова.
///
/// Считаем именно символы, а не байты: русский текст в UTF-8 занимает по два
/// байта на букву, и обрезка по байтам разрезала бы слово посередине.
pub fn excerpt(markdown: &str, max_chars: usize) -> String {
    let text = to_plain_text(markdown);
    if text.chars().count() <= max_chars {
        return text;
    }

    let mut out = String::with_capacity(max_chars + 1);
    for word in text.split_whitespace() {
        if out.chars().count() + word.chars().count() + 1 > max_chars {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }

    if out.is_empty() {
        // Единственное слово длиннее лимита — режем по символам.
        out = text.chars().take(max_chars).collect();
    }
    out.push('…');
    out
}

/// Схлопывает пробелы и убирает пробел перед знаком препинания.
///
/// Разрывы тегов дают лишний пробел («[ссылка](…) .» → «ссылка .»), потому что
/// `Event::End` вставляется как разделитель. Для плоского текста это мусор.
fn collapse_whitespace(s: &str) -> String {
    let joined = s.split_whitespace().collect::<Vec<_>>().join(" ");

    const NO_SPACE_BEFORE: [&str; 9] = [".", ",", "!", "?", ";", ":", ")", "]", "»"];
    let mut out = joined;
    for punct in NO_SPACE_BEFORE {
        out = out.replace(&format!(" {punct}"), punct);
    }
    out
}

/// Убирает из потока событий всё, что может выполниться в браузере.
fn sanitize_event(event: Event<'_>) -> Event<'_> {
    match event {
        // Сырой HTML становится текстом: markdown-возможностей автору хватает,
        // а <script> из внешнего источника — нет.
        Event::Html(raw) | Event::InlineHtml(raw) => Event::Text(raw),
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Link {
            link_type,
            dest_url: sanitize_url(dest_url),
            title,
            id,
        }),
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Image {
            link_type,
            dest_url: sanitize_url(dest_url),
            title,
            id,
        }),
        other => other,
    }
}

/// Заменяет опасные схемы ссылок на безобидный `#`.
///
/// `data:` блокируем целиком и включая `data:image/svg+xml`: SVG умеет нести
/// скрипт, а пользы от inline-картинок в статье почти нет.
fn sanitize_url(url: CowStr<'_>) -> CowStr<'_> {
    let lowered = url.trim_start().to_ascii_lowercase();
    let dangerous = lowered.starts_with("javascript:")
        || lowered.starts_with("vbscript:")
        || lowered.starts_with("data:");

    if dangerous {
        CowStr::Borrowed("#")
    } else {
        url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_basic_markdown() {
        let html = render("# Заголовок\n\nТекст со **скобками** и `кодом`.");
        assert!(html.contains("<h1>Заголовок</h1>"));
        assert!(html.contains("<strong>скобками</strong>"));
        assert!(html.contains("<code>кодом</code>"));
    }

    #[test]
    fn renders_tables_and_strikethrough() {
        let html = render("| a | b |\n|---|---|\n| 1 | 2 |\n\n~~нет~~");
        assert!(html.contains("<table>"));
        assert!(html.contains("<del>нет</del>"));
    }

    #[test]
    fn escapes_raw_html_instead_of_executing_it() {
        // Главная проверка XSS: <script> не должен попасть в вывод как тег.
        let html = render("Привет <script>alert('xss')</script>");
        assert!(!html.contains("<script>"), "сырой script просочился: {html}");
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn neutralises_javascript_and_data_urls() {
        let html = render("[клик](javascript:alert(1))");
        assert!(!html.contains("javascript:"), "js-ссылка просочилась: {html}");

        let html = render("![x](data:text/html;base64,PHNjcmlwdD4=)");
        assert!(!html.contains("data:"), "data-ссылка просочилась: {html}");
    }

    #[test]
    fn keeps_normal_links_and_images() {
        let html = render("[сайт](https://example.com) ![alt](/img/a.png)");
        assert!(html.contains(r#"href="https://example.com""#));
        assert!(html.contains(r#"src="/img/a.png""#));
    }

    #[test]
    fn plain_text_drops_markup() {
        let text = to_plain_text("# Заголовок\n\nТекст с **жирным** и [ссылкой](https://x.ru).");
        assert_eq!(text, "Заголовок Текст с жирным и ссылкой.");
    }

    #[test]
    fn reading_time_is_at_least_one_minute() {
        assert_eq!(reading_time_minutes(""), 1);
        assert_eq!(reading_time_minutes("одно слово"), 1);
        // 360 слов при 180 сл/мин — ровно две минуты.
        let body = "слово ".repeat(360);
        assert_eq!(reading_time_minutes(&body), 2);
    }

    #[test]
    fn excerpt_truncates_on_word_boundary() {
        let long = "Первое второе третье четвёртое пятое";
        let short = excerpt(long, 20);
        assert!(short.ends_with('…'));
        // Обрезка по словам: последнее слово не разрублено.
        assert!(short.chars().count() <= 21);
        assert!(short.starts_with("Первое"));
    }

    #[test]
    fn excerpt_keeps_short_text_intact() {
        assert_eq!(excerpt("Коротко", 100), "Коротко");
    }
}

//! Синдикация блога: RSS-лента и карта сайта.
//!
//! # Зачем это кросспостингу
//!
//! RSS — второй (после outbox) канал для n8n. Outbox даёт точные события
//! публикации, а RSS отдаёт **полный текст** статьи в `content:encoded`, поэтому
//! нода RSS Feed Trigger в n8n может собрать пост для Telegram/VK/Habr, не
//! делая второй запрос к сайту.
//!
//! Обе ленты строятся из [`PublishedArticles`], то есть черновики и архив в них
//! попасть не могут по построению.

use crate::services::article::{Article, ArticleHead, PublishedArticles};

/// Статические страницы сайта, которые всегда должны быть в карте сайта.
const STATIC_PAGES: &[&str] = &["/", "/services", "/projects", "/blog", "/chat", "/contact"];

/// Название сайта в лентах.
const FEED_TITLE: &str = "inteli.dev — блог";
/// Описание сайта в лентах.
const FEED_DESCRIPTION: &str =
    "Заметки о приватных AI-системах, локальном инференсе, MCP-серверах и fullstack-разработке.";

/// Собирает RSS 2.0 ленту опубликованных статей.
pub fn rss_xml(articles: &[Article], public_url: &str) -> String {
    let base = public_url.trim_end_matches('/');
    let mut out = String::with_capacity(1024 + articles.len() * 2048);

    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(
        "<rss version=\"2.0\" \
         xmlns:atom=\"http://www.w3.org/2005/Atom\" \
         xmlns:content=\"http://purl.org/rss/1.0/modules/content/\">\n",
    );
    out.push_str("  <channel>\n");
    out.push_str(&format!("    <title>{}</title>\n", escape_xml(FEED_TITLE)));
    out.push_str(&format!("    <link>{base}/blog</link>\n"));
    out.push_str(&format!(
        "    <description>{}</description>\n",
        escape_xml(FEED_DESCRIPTION)
    ));
    out.push_str("    <language>ru</language>\n");
    out.push_str(&format!(
        "    <atom:link href=\"{base}/rss.xml\" rel=\"self\" type=\"application/rss+xml\"/>\n"
    ));

    for article in articles {
        let url = format!("{base}/blog/{}", article.slug);
        out.push_str("    <item>\n");
        out.push_str(&format!("      <title>{}</title>\n", escape_xml(&article.title)));
        out.push_str(&format!("      <link>{}</link>\n", escape_xml(&url)));
        out.push_str(&format!(
            "      <guid isPermaLink=\"true\">{}</guid>\n",
            escape_xml(&url)
        ));
        if let Some(date) = article.published_at.as_deref().and_then(rfc2822) {
            out.push_str(&format!("      <pubDate>{date}</pubDate>\n"));
        }
        out.push_str(&format!(
            "      <description>{}</description>\n",
            escape_xml(&article.description())
        ));
        for tag in &article.tags {
            out.push_str(&format!("      <category>{}</category>\n", escape_xml(tag)));
        }
        // Полный HTML: n8n забирает готовый пост одним запросом.
        out.push_str(&format!(
            "      <content:encoded><![CDATA[{}]]></content:encoded>\n",
            cdata(&article.html())
        ));
        out.push_str("    </item>\n");
    }

    out.push_str("  </channel>\n</rss>\n");
    out
}

/// Собирает карту сайта: статические страницы + опубликованные статьи.
pub fn sitemap_xml(articles: &[ArticleHead], public_url: &str) -> String {
    let base = public_url.trim_end_matches('/');
    let mut out = String::with_capacity(512 + articles.len() * 256);

    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

    for page in STATIC_PAGES {
        out.push_str(&format!("  <url><loc>{base}{page}</loc></url>\n"));
    }

    for article in articles {
        out.push_str(&format!(
            "  <url><loc>{base}/blog/{}</loc>{}</url>\n",
            escape_xml(&article.slug),
            article
                .published_at
                .as_deref()
                .map(w3c_date)
                .map(|d| format!("<lastmod>{d}</lastmod>"))
                .unwrap_or_default()
        ));
    }

    out.push_str("</urlset>\n");
    out
}

/// Экранирует текст для XML-узла.
pub fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Готовит произвольный текст для вставки внутрь `CDATA`.
///
/// Последовательность `]]>` закрывает секцию, поэтому её разрывают на две:
/// `]]]]><![CDATA[>`. Так HTML из статьи не может «вырваться» из CDATA.
fn cdata(s: &str) -> String {
    s.replace("]]>", "]]]]><![CDATA[>")
}

/// RFC3339 → RFC822 (формат `pubDate` в RSS 2.0).
fn rfc2822(rfc3339: &str) -> Option<String> {
    chrono::DateTime::parse_from_rfc3339(rfc3339)
        .ok()
        .map(|dt| dt.to_rfc2822())
}

/// RFC3339 → `YYYY-MM-DD` для `lastmod` в карте сайта.
fn w3c_date(rfc3339: &str) -> String {
    rfc3339.chars().take(10).collect()
}

/// Снимок лент для текущего состояния сайта.
pub fn current_rss(public_url: &str) -> String {
    rss_xml(&PublishedArticles::list(), public_url)
}

/// Снимок карты сайта для текущего состояния сайта.
pub fn current_sitemap(public_url: &str) -> String {
    sitemap_xml(&PublishedArticles::heads(), public_url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::article::ArticleStatus;

    fn article(slug: &str, title: &str, body: &str) -> Article {
        Article {
            id: 1,
            slug: slug.into(),
            title: title.into(),
            summary: "Кратко".into(),
            body_markdown: body.into(),
            cover_image_url: None,
            tags: vec!["rust".into(), "ai".into()],
            status: ArticleStatus::Published,
            published_at: Some("2026-09-24T12:00:00Z".into()),
            created_at: "2026-09-24T12:00:00Z".into(),
            updated_at: "2026-09-24T12:00:00Z".into(),
            source: "admin".into(),
            external_id: None,
            canonical_url: None,
        }
    }

    #[test]
    fn xml_escaping_covers_all_entities() {
        assert_eq!(
            escape_xml(r#"<a href="x">Tom & Jerry's</a>"#),
            "&lt;a href=&quot;x&quot;&gt;Tom &amp; Jerry&apos;s&lt;/a&gt;"
        );
    }

    #[test]
    fn rss_contains_items_with_full_content() {
        let xml = rss_xml(&[article("hello", "Привет & мир", "**жирный**")], "https://x.ru");

        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<title>Привет &amp; мир</title>"));
        assert!(xml.contains("<link>https://x.ru/blog/hello</link>"));
        assert!(xml.contains("<category>rust</category>"));
        assert!(xml.contains("<pubDate>"));
        // Полный текст статьи — то, ради чего n8n читает ленту.
        assert!(xml.contains("<content:encoded><![CDATA["));
        assert!(xml.contains("<strong>жирный</strong>"));
    }

    #[test]
    fn cdata_splits_the_terminator() {
        // Классический приём: `]]>` разрывается на границе двух CDATA-секций,
        // а при разборе текст собирается обратно.
        assert_eq!(cdata("a]]>b"), "a]]]]><![CDATA[>b");
    }

    #[test]
    fn article_body_cannot_close_cdata() {
        // Тело статьи не должно уметь закрыть CDATA и дописать свой XML.
        // Рендерер экранирует `<` и `>`, поэтому собрать `]]>` из текста
        // статьи физически нечем — это второй барьер после самого `cdata`.
        let evil = article("xss", "t", "текст ]]> <script>alert(1)</script>");
        let xml = rss_xml(&[evil], "https://x.ru");

        assert!(!xml.contains("]]><script>"), "CDATA закрыта снаружи: {xml}");
        assert!(!xml.contains("<script>"), "скрипт просочился в ленту: {xml}");
    }

    #[test]
    fn empty_feed_is_still_valid_rss() {
        let xml = rss_xml(&[], "https://x.ru");
        assert!(xml.contains("<channel>"));
        assert!(xml.contains("</rss>"));
        assert!(!xml.contains("<item>"));
    }

    #[test]
    fn sitemap_lists_static_pages_and_articles() {
        let head = article("hello", "t", "body").head();
        let xml = sitemap_xml(&[head], "https://x.ru");

        assert!(xml.contains("<loc>https://x.ru/</loc>"));
        assert!(xml.contains("<loc>https://x.ru/blog</loc>"));
        assert!(xml.contains("<loc>https://x.ru/blog/hello</loc>"));
        assert!(xml.contains("<lastmod>2026-09-24</lastmod>"));
    }

    #[test]
    fn public_url_trailing_slash_does_not_double_up() {
        let xml = rss_xml(&[article("a", "t", "b")], "https://x.ru/");
        assert!(xml.contains("https://x.ru/blog/a"));
        assert!(!xml.contains("https://x.ru//blog"));
    }
}

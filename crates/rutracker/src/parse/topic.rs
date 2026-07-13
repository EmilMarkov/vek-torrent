//! Разбор страницы раздачи `viewtopic.php`.

use std::sync::LazyLock;

use regex::Regex;
use scraper::{Html, Selector};
use url::Url;

use crate::{
    Result,
    error::Error,
    models::{ForumRef, TopicPage, TorrentStats},
    parse::common::{collapse_whitespace, element_text, first_int, parse_size_text, query_param},
    parse::sanitize::sanitize_post_body,
};

static TITLE: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("h1.maintitle a, h1.maintitle").expect("selector"));
static BREADCRUMB: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("td.nav a, .t-breadcrumb-top a").expect("selector"));
static MAGNET: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("a.magnet-link").expect("selector"));
static DL_LINK: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"a[href*="dl.php"]"#).expect("selector"));
static POST_BODY: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.post_body").expect("selector"));
static AUTHOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("p.nick-author, p.nick").expect("selector"));
static SEED: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("span.seed b, span.seed").expect("selector"));
static LEECH: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("span.leech b, span.leech").expect("selector"));

static RE_SIZE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Размер:\s*([\d.,]+\s*(?:[KMGT]?B|[КМГТ]?Б))").expect("regex"));
static RE_COMPLETED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Скачан:\s*([\d\s.,\u{a0}]+)\s*раз").expect("regex"));
static RE_REGISTERED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Зарегистрирован:\s*(\S+(?:\s+\d{1,2}:\d{2})?)").expect("regex"));

/// Разбирает страницу раздачи.
pub fn parse_topic_page(html: &str, id: u64, base: &Url) -> Result<TopicPage> {
    let doc = Html::parse_document(html);

    let title = doc
        .select(&TITLE)
        .next()
        .map(element_text)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| Error::parse("заголовок раздачи не найден"))?;

    let mut forum_path: Vec<ForumRef> = Vec::new();
    for a in doc.select(&BREADCRUMB) {
        let Some(href) = a.value().attr("href") else {
            continue;
        };
        if !href.contains("viewforum.php") {
            continue;
        }
        let Some(forum_id) = query_param(href, "f").and_then(|v| v.parse().ok()) else {
            continue;
        };
        let name = element_text(a);
        if name.is_empty() {
            continue;
        }
        if forum_path.last().map(|f: &ForumRef| f.id) != Some(forum_id) {
            forum_path.push(ForumRef { id: forum_id, name });
        }
    }

    let magnet = doc
        .select(&MAGNET)
        .next()
        .and_then(|a| a.value().attr("href"))
        .filter(|href| href.starts_with("magnet:"))
        .map(str::to_owned);

    let has_torrent_file = doc.select(&DL_LINK).next().is_some();

    let body_html = doc
        .select(&POST_BODY)
        .next()
        .map(|el| sanitize_post_body(&el.inner_html(), base))
        .unwrap_or_default();

    // Автор первого поста (ник рядом с телом раздачи).
    let author = doc
        .select(&AUTHOR)
        .next()
        .map(element_text)
        .filter(|a| !a.is_empty());

    let page_text = collapse_whitespace(&doc.root_element().text().collect::<String>());

    let stats = TorrentStats {
        size_bytes: RE_SIZE
            .captures(&page_text)
            .and_then(|c| parse_size_text(&c[1])),
        seeders: doc
            .select(&SEED)
            .next()
            .and_then(|el| first_int(&element_text(el))),
        leechers: doc
            .select(&LEECH)
            .next()
            .and_then(|el| first_int(&element_text(el))),
        completed: RE_COMPLETED
            .captures(&page_text)
            .and_then(|c| first_int(&c[1])),
        registered: RE_REGISTERED
            .captures(&page_text)
            .map(|c| c[1].trim().to_owned()),
    };

    Ok(TopicPage {
        id,
        title,
        forum_path,
        author,
        magnet,
        has_torrent_file,
        stats,
        body_html,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../tests/fixtures/topic_page.html");

    fn base() -> Url {
        Url::parse("https://rutracker.org/forum/").unwrap()
    }

    #[test]
    fn parses_topic_fixture() {
        let topic = parse_topic_page(FIXTURE, 6335144, &base()).unwrap();

        assert!(topic.title.contains("Linux Mint 21.3"));
        assert_eq!(topic.forum_path.len(), 2);
        assert_eq!(topic.forum_path[0].id, 2093);
        assert_eq!(topic.forum_path[1].id, 1379);

        assert_eq!(topic.author.as_deref(), Some("mint_keeper"));
        assert!(topic.magnet.as_deref().unwrap().starts_with("magnet:?xt="));
        assert!(topic.has_torrent_file);

        assert_eq!(topic.stats.size_bytes, Some(734_003_200));
        assert_eq!(topic.stats.seeders, Some(152));
        assert_eq!(topic.stats.leechers, Some(3));
        assert_eq!(topic.stats.completed, Some(2404));
        assert_eq!(topic.stats.registered.as_deref(), Some("10-Сен-24"));

        // Тело — санированный HTML в родной разметке rutracker: спойлер со
        // скриншотами и блок кода сохранены, скрипты вырезаны.
        assert!(!topic.body_html.is_empty());
        assert!(topic.body_html.contains("sp-wrap"));
        assert!(topic.body_html.contains("Скриншоты"));
        assert!(topic.body_html.contains("c-body"));
        assert!(!topic.body_html.contains("<script"));
    }

    #[test]
    fn missing_title_is_error() {
        assert!(parse_topic_page("<html><body></body></html>", 1, &base()).is_err());
    }
}

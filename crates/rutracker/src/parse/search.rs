//! Разбор страницы результатов поиска `tracker.php`.

use std::sync::LazyLock;

use regex::Regex;
use scraper::{ElementRef, Html, Selector};

use crate::{
    Result,
    error::Error,
    models::{ApprovalStatus, ForumRef, SearchPage, SearchResult},
    parse::common::{element_text, first_int, has_class, parse_size_text, query_param},
};

static ROW: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("table#tor-tbl tr.hl-tr, tr.tCenter.hl-tr").expect("selector")
});
static TD: LazyLock<Selector> = LazyLock::new(|| Selector::parse("td").expect("selector"));
static SIZE_LINK: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("a.tr-dl").expect("selector"));
static TOPIC_LINK: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("td.t-title-col a, a.tLink").expect("selector"));
static FORUM_LINK: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("td.f-name-col a").expect("selector"));
static AUTHOR_LINK: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("td.u-name-col a, .u-name a").expect("selector"));
static ICON: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("span.tor-icon, img.tor-icon").expect("selector"));
static SIZE_CELL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("td.tor-size").expect("selector"));
static SEED: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("b.seedmed, span.seedmed, td.seedmed").expect("selector"));
static LEECH: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("b.leechmed, span.leechmed, td.leechmed").expect("selector"));
static NUMBER_CELL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("td.number-format").expect("selector"));
static U_TAG: LazyLock<Selector> = LazyLock::new(|| Selector::parse("u").expect("selector"));

static RE_TOTAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Результатов поиска:\s*(\d+)").expect("regex"));
static RE_SEARCH_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"search_id=([0-9A-Za-z]+)").expect("regex"));

/// Разбирает страницу результатов поиска.
pub fn parse_search_page(html: &str, offset: u32) -> Result<SearchPage> {
    let doc = Html::parse_document(html);

    let items: Vec<SearchResult> = doc.select(&ROW).filter_map(parse_row).collect();

    let total_found = RE_TOTAL
        .captures(html)
        .and_then(|c| c[1].parse().ok())
        .unwrap_or(u64::from(offset) + items.len() as u64);

    let search_id = RE_SEARCH_ID.captures(html).map(|c| c[1].to_owned());

    // Страница без таблицы результатов и без счётчика — что-то другое
    // (заглушка, техработы, антибот).
    if items.is_empty() && !html.contains("tor-tbl") && RE_TOTAL.captures(html).is_none() {
        return Err(Error::parse(
            "страница результатов поиска не распознана (возможно, изменилась разметка)",
        ));
    }

    Ok(SearchPage {
        items,
        total_found,
        offset,
        search_id,
    })
}

fn parse_row(row: ElementRef<'_>) -> Option<SearchResult> {
    let link = row.select(&TOPIC_LINK).next()?;
    let href = link.value().attr("href")?;
    let topic_id: u64 = query_param(href, "t")?.parse().ok()?;
    let title = element_text(link);
    if title.is_empty() {
        return None;
    }

    let forum = row.select(&FORUM_LINK).next().and_then(|a| {
        let id = query_param(a.value().attr("href")?, "f")?.parse().ok()?;
        let name = element_text(a);
        (!name.is_empty()).then_some(ForumRef { id, name })
    });

    let author = row
        .select(&AUTHOR_LINK)
        .next()
        .map(element_text)
        .filter(|s| !s.is_empty());

    let approval = row
        .select(&ICON)
        .next()
        .and_then(|el| el.value().attr("class"))
        .map(ApprovalStatus::from_icon_classes)
        .unwrap_or(ApprovalStatus::Unknown);

    let size_bytes = row
        .select(&SIZE_CELL)
        .next()
        .and_then(|td| {
            // Основной путь — скрытый <u> с точным числом байт; запасной —
            // человекочитаемый текст ссылки («2.67 GB»), без текста <u>.
            td.select(&U_TAG)
                .next()
                .and_then(|u| element_text(u).parse().ok())
                .or_else(|| {
                    td.select(&SIZE_LINK)
                        .next()
                        .and_then(|a| parse_size_text(&element_text(a)))
                })
        })
        .unwrap_or(0);

    let seeders = row
        .select(&SEED)
        .next()
        .and_then(|el| first_int(&element_text(el)))
        .unwrap_or(0);

    let leechers = row
        .select(&LEECH)
        .next()
        .and_then(|el| first_int(&element_text(el)))
        .unwrap_or(0);

    let downloads = row
        .select(&NUMBER_CELL)
        .find(|td| {
            !has_class(*td, "seedmed") && !has_class(*td, "leechmed") && !has_class(*td, "tor-size")
        })
        .and_then(|td| first_int(&element_text(td)))
        .unwrap_or(0);

    // Дата регистрации — скрытый <u> (unix-время) в последней ячейке строки.
    let added_unix = row
        .select(&TD)
        .last()
        .and_then(|td| td.select(&U_TAG).next())
        .and_then(|u| element_text(u).parse::<i64>().ok())
        .unwrap_or(0);

    Some(SearchResult {
        topic_id,
        title,
        forum,
        author,
        size_bytes,
        seeders,
        leechers,
        downloads,
        added_unix,
        approval,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../tests/fixtures/search_results.html");

    #[test]
    fn parses_fixture_rows() {
        let page = parse_search_page(FIXTURE, 0).expect("страница должна разобраться");
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.total_found, 2);
        assert_eq!(page.search_id.as_deref(), Some("Af1B2c3D"));

        let first = &page.items[0];
        assert_eq!(first.topic_id, 6335144);
        assert!(first.title.contains("Linux Mint"));
        assert_eq!(first.forum.as_ref().map(|f| f.id), Some(1379));
        assert_eq!(first.size_bytes, 2_863_311_530);
        assert_eq!(first.seeders, 152);
        assert_eq!(first.leechers, 3);
        assert_eq!(first.downloads, 2404);
        assert_eq!(first.added_unix, 1_726_000_000);
        assert_eq!(first.approval, ApprovalStatus::Approved);
        assert_eq!(first.author.as_deref(), Some("mint_keeper"));

        let second = &page.items[1];
        assert_eq!(second.topic_id, 6100200);
        assert_eq!(second.approval, ApprovalStatus::NotApproved);
    }

    #[test]
    fn empty_results_are_ok_when_marker_present() {
        let html = r#"<html><body><table id="tor-tbl"><tbody></tbody></table>
            <p>Результатов поиска: 0</p></body></html>"#;
        let page = parse_search_page(html, 0).unwrap();
        assert!(page.items.is_empty());
        assert_eq!(page.total_found, 0);
        assert!(!page.has_more());
    }

    #[test]
    fn unknown_page_is_error() {
        assert!(parse_search_page("<html><body>заглушка</body></html>", 0).is_err());
    }
}

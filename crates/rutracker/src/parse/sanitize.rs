//! Санитизация HTML первого поста раздачи.
//!
//! Приложение показывает пост в родной разметке rutracker (classes
//! `post-*`, `sp-*`, `q-*`, `c-*`, `postImg`…), чтобы авторское оформление
//! (обтекание картинок, таблицы, выравнивание) сохранялось 1:1 — фронтенд
//! стилизует эти классы под тёмную тему. Безопасность обеспечивает строгий
//! whitelist: разрешены только известные теги, классы rutracker и небольшой
//! набор CSS-свойств в `style`; скрипты, обработчики и опасные схемы URL
//! вырезаются, относительные ссылки становятся абсолютными.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use ammonia::{Builder, UrlRelative};
use url::Url;

/// Санирует внутренний HTML `div.post_body`.
pub fn sanitize_post_body(html: &str, base: &Url) -> String {
    builder(base).clean(html).to_string()
}

/// Классы span из разметки rutracker (BB-коды).
const SPAN_CLASSES: &[&str] = &[
    "post-b",
    "post-i",
    "post-u",
    "post-s",
    "post-br",
    "post-hr",
    "post-align",
    "p-color",
    "post-font-serif1",
    "post-font-serif2",
    "post-font-sans1",
    "post-font-sans2",
    "post-font-sans3",
    "post-font-mono1",
    "post-font-mono2",
    "post-font-cursive1",
    "post-font-impact",
];

/// Классы контейнеров: спойлеры, цитаты, код, box-обёртки.
const DIV_CLASSES: &[&str] = &[
    "sp-wrap",
    "sp-head",
    "sp-body",
    "sp-fold",
    "q-wrap",
    "q-head",
    "q",
    "c-wrap",
    "c-head",
    "c-body",
    "post-box",
    "post-box-default",
    "post-box-left",
    "post-box-right",
    "post-box-center",
    "post-align",
    "post-indent",
    "folded",
    "unfolded",
];

/// Классы изображений (включая `var.postImg` до подстановки `img` на фронте).
const IMG_CLASSES: &[&str] = &[
    "postImg",
    "postImgAligned",
    "img-left",
    "img-right",
    "img-center",
    "postImg10",
    "postImg15",
    "postImg20",
    "postImg25",
    "postImg30",
    "postImg40",
    "postImg50",
    "postImg60",
    "postImg1em",
    "smile",
    "smile-img",
    "post-img-broken",
];

fn builder(base: &Url) -> Builder<'static> {
    let mut builder = Builder::new();

    builder
        .tags(HashSet::from_iter([
            "a",
            "b",
            "strong",
            "i",
            "em",
            "u",
            "s",
            "strike",
            "del",
            "br",
            "hr",
            "span",
            "div",
            "p",
            "var",
            "img",
            "ul",
            "ol",
            "li",
            "table",
            "tbody",
            "thead",
            "tfoot",
            "tr",
            "td",
            "th",
            "pre",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "blockquote",
            "sub",
            "sup",
            "center",
            "dl",
            "dt",
            "dd",
        ]))
        // title нужен var.postImg (там URL картинки) и всплывающим подсказкам.
        .generic_attributes(HashSet::from_iter(["style", "title"]))
        .tag_attributes(HashMap::from_iter([
            ("a", HashSet::from_iter(["href"])),
            ("img", HashSet::from_iter(["src", "alt", "width", "height"])),
            (
                "td",
                HashSet::from_iter(["colspan", "rowspan", "align", "valign"]),
            ),
            (
                "th",
                HashSet::from_iter(["colspan", "rowspan", "align", "valign"]),
            ),
            ("div", HashSet::from_iter(["align"])),
            ("p", HashSet::from_iter(["align"])),
        ]))
        .allowed_classes(HashMap::from_iter([
            ("span", HashSet::from_iter(SPAN_CLASSES.iter().copied())),
            ("div", HashSet::from_iter(DIV_CLASSES.iter().copied())),
            ("var", HashSet::from_iter(IMG_CLASSES.iter().copied())),
            ("img", HashSet::from_iter(IMG_CLASSES.iter().copied())),
            (
                "a",
                HashSet::from_iter(["postLink", "magnet-link", "med", "small"]),
            ),
            ("ul", HashSet::from_iter(["post-ul"])),
            ("ol", HashSet::from_iter(["post-ol"])),
            ("pre", HashSet::from_iter(["post-pre", "post-nfo"])),
        ]))
        .url_schemes(HashSet::from_iter(["http", "https", "magnet"]))
        .url_relative(UrlRelative::RewriteWithBase(base.clone()))
        .attribute_filter(|_, attribute, value| match attribute {
            "style" => sanitize_style(value).map(Cow::Owned),
            _ => Some(Cow::Borrowed(value)),
        });

    builder
}

/// CSS-свойства, разрешённые в inline-стилях авторского оформления.
const ALLOWED_STYLE_PROPS: &[&str] = &[
    "color",
    "font-size",
    "font-family",
    "font-weight",
    "font-style",
    "text-decoration",
    "text-decoration-line",
    "text-align",
    "line-height",
    "vertical-align",
];

/// Фильтрует объявления `style`: только whitelisted свойства с безопасными
/// значениями (без `url(...)`, выражений и вложенных блоков).
fn sanitize_style(style: &str) -> Option<String> {
    let mut kept: Vec<String> = Vec::new();
    for declaration in style.split(';') {
        let Some((prop, value)) = declaration.split_once(':') else {
            continue;
        };
        let prop = prop.trim().to_ascii_lowercase();
        let value = value.trim();
        if !ALLOWED_STYLE_PROPS.contains(&prop.as_str()) {
            continue;
        }
        if value.is_empty() || !value.chars().all(is_safe_style_char) {
            continue;
        }
        kept.push(format!("{prop}: {value}"));
    }
    (!kept.is_empty()).then(|| kept.join("; "))
}

/// Безопасные символы значения CSS-свойства. Двоеточие исключено, поэтому
/// `url(data:…)` и прочие схемы в значение не проходят.
fn is_safe_style_char(c: char) -> bool {
    c.is_alphanumeric()
        || matches!(
            c,
            '#' | '(' | ')' | ',' | '.' | '%' | ' ' | '-' | '"' | '\''
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("https://rutracker.org/forum/").unwrap()
    }

    fn clean(html: &str) -> String {
        sanitize_post_body(html, &base())
    }

    #[test]
    fn strips_scripts_and_handlers() {
        let out = clean(r#"<script>alert(1)</script><span onclick="x()">Текст</span>"#);
        assert!(!out.contains("script"));
        assert!(!out.contains("onclick"));
        assert!(out.contains("Текст"));
    }

    #[test]
    fn keeps_rutracker_markup() {
        let out = clean(
            r#"<span class="post-b">Жанр</span>: Survival Horror<br>
            <var class="postImg postImgAligned img-right" title="https://i.example/poster.jpg">&#10;</var>
            <div class="sp-wrap"><div class="sp-head folded"><span>Скриншоты</span></div>
            <div class="sp-body"><var class="postImg" title="https://i.example/1.png">&#10;</var></div></div>"#,
        );
        assert!(out.contains(r#"class="post-b""#));
        assert!(out.contains(r#"class="postImg postImgAligned img-right""#));
        assert!(out.contains(r#"title="https://i.example/poster.jpg""#));
        assert!(out.contains("sp-head"));
        assert!(out.contains("sp-body"));
    }

    #[test]
    fn filters_style_properties() {
        let out = clean(
            r#"<span style="font-size: 24px; position: fixed; color: red; background: url(https://e/x)">Т</span>"#,
        );
        assert!(out.contains("font-size: 24px"));
        assert!(out.contains("color: red"));
        assert!(!out.contains("position"));
        assert!(!out.contains("url("));
    }

    #[test]
    fn drops_unknown_classes() {
        let out = clean(
            r#"<div class="fixed inset-0 sp-wrap">x</div><span class="post-b evil">y</span>"#,
        );
        assert!(out.contains(r#"class="sp-wrap""#));
        assert!(out.contains(r#"class="post-b""#));
        assert!(!out.contains("fixed"));
        assert!(!out.contains("evil"));
    }

    #[test]
    fn rewrites_relative_urls() {
        let out = clean(r#"<a href="viewtopic.php?t=555">Раздача</a>"#);
        assert!(out.contains(r#"href="https://rutracker.org/forum/viewtopic.php?t=555""#));
    }

    #[test]
    fn blocks_javascript_urls() {
        let out = clean(r#"<a href="javascript:alert(1)">x</a>"#);
        assert!(!out.contains("javascript"));
    }

    #[test]
    fn keeps_tables_and_alignment() {
        let out = clean(
            r#"<table><tr><td align="center">Ячейка</td></tr></table><span class="post-align" style="text-align: center;">По центру</span>"#,
        );
        assert!(out.contains("<table"));
        assert!(out.contains(r#"align="center""#));
        assert!(out.contains("text-align: center"));
    }
}

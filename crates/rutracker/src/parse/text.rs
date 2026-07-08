//! Преобразование HTML первого поста раздачи в блочную модель
//! ([`ContentBlock`]) — собственный безопасный рендер вместо чужого HTML.

use std::sync::LazyLock;

use scraper::{ElementRef, Node, Selector};
use url::Url;

use crate::{
    models::{ContentBlock, Inline},
    parse::common::{element_text, has_class, query_param},
};

static SP_HEAD: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".sp-head").expect("selector"));
static SP_BODY: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".sp-body").expect("selector"));
static Q_INNER: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".q").expect("selector"));
static Q_HEAD: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".q-head").expect("selector"));
static C_BODY: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".c-body").expect("selector"));

/// Строит блоки из элемента `div.post_body`.
pub fn blocks_from_post_body(root: ElementRef<'_>, base: &Url) -> Vec<ContentBlock> {
    let mut builder = Builder::default();
    walk_children(root, &Marks::default(), &mut builder, base);
    builder.finish()
}

/// Активные строчные стили.
#[derive(Debug, Default, Clone)]
struct Marks {
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    color: Option<String>,
}

/// Аккумулятор блоков и текущего параграфа.
#[derive(Default)]
struct Builder {
    blocks: Vec<ContentBlock>,
    current: Vec<Inline>,
}

impl Builder {
    fn push_text(&mut self, raw: &str, marks: &Marks) {
        let mut text = String::with_capacity(raw.len());
        let mut prev_space = false;
        for ch in raw.chars() {
            if ch.is_whitespace() {
                if !prev_space {
                    text.push(' ');
                }
                prev_space = true;
            } else {
                text.push(ch);
                prev_space = false;
            }
        }
        // В начале параграфа и после переноса ведущий пробел не нужен.
        if self.current.is_empty() || matches!(self.current.last(), Some(Inline::Break)) {
            while text.starts_with(' ') {
                text.remove(0);
            }
        }
        // Не дублируем пробел на стыке соседних фрагментов.
        if let Some(Inline::Text { text: last, .. }) = self.current.last()
            && last.ends_with(' ')
            && text.starts_with(' ')
        {
            text.remove(0);
        }
        if text.is_empty() {
            return;
        }

        // Склеиваем соседние фрагменты с одинаковыми стилями.
        if let Some(Inline::Text {
            text: last,
            bold,
            italic,
            underline,
            strike,
            color,
        }) = self.current.last_mut()
            && *bold == marks.bold
            && *italic == marks.italic
            && *underline == marks.underline
            && *strike == marks.strike
            && color.as_deref() == marks.color.as_deref()
        {
            last.push_str(&text);
            return;
        }

        self.current.push(Inline::Text {
            text,
            bold: marks.bold,
            italic: marks.italic,
            underline: marks.underline,
            strike: marks.strike,
            color: marks.color.clone(),
        });
    }

    fn push_inline(&mut self, inline: Inline) {
        self.current.push(inline);
    }

    fn push_break(&mut self) {
        // Ведущие и повторные переносы не накапливаем.
        match self.current.last() {
            None => {}
            Some(Inline::Break) => {}
            _ => self.current.push(Inline::Break),
        }
    }

    fn flush(&mut self) {
        while matches!(self.current.last(), Some(Inline::Break)) {
            self.current.pop();
        }
        if !self.current.is_empty() {
            let inlines = std::mem::take(&mut self.current);
            self.blocks.push(ContentBlock::Paragraph { inlines });
        }
    }

    fn push_block(&mut self, block: ContentBlock) {
        self.flush();
        self.blocks.push(block);
    }

    fn finish(mut self) -> Vec<ContentBlock> {
        self.flush();
        self.blocks
    }
}

fn walk_children(el: ElementRef<'_>, marks: &Marks, builder: &mut Builder, base: &Url) {
    for child in el.children() {
        match child.value() {
            Node::Text(text) => builder.push_text(&text.text, marks),
            Node::Element(_) => {
                if let Some(child_el) = ElementRef::wrap(child) {
                    handle_element(child_el, marks, builder, base);
                }
            }
            _ => {}
        }
    }
}

fn handle_element(el: ElementRef<'_>, marks: &Marks, builder: &mut Builder, base: &Url) {
    let tag = el.value().name();
    match tag {
        "br" => builder.push_break(),
        "hr" => builder.push_block(ContentBlock::Hr),

        "var" if has_class(el, "postImg") => {
            if let Some(src) = el.value().attr("title") {
                builder.push_block(ContentBlock::Image {
                    src: resolve_href(src, base),
                });
            }
        }

        "img" => {
            if has_class(el, "smile") || has_class(el, "smile-img") {
                return;
            }
            if let Some(src) = el.value().attr("src") {
                builder.push_block(ContentBlock::Image {
                    src: resolve_href(src, base),
                });
            }
        }

        "a" => handle_link(el, marks, builder, base),

        "ul" | "ol" => {
            let ordered = tag == "ol";
            let mut items = Vec::new();
            for li in el.children() {
                let Some(li_el) = ElementRef::wrap(li) else {
                    continue;
                };
                if li_el.value().name() != "li" {
                    continue;
                }
                let mut item_builder = Builder::default();
                walk_children(li_el, marks, &mut item_builder, base);
                let blocks = item_builder.finish();
                if !blocks.is_empty() {
                    items.push(blocks);
                }
            }
            if !items.is_empty() {
                builder.push_block(ContentBlock::List { ordered, items });
            }
        }

        "div" | "p" => {
            if has_class(el, "sp-wrap") {
                handle_spoiler(el, marks, builder, base);
            } else if has_class(el, "q-wrap") {
                handle_quote(el, marks, builder, base);
            } else if has_class(el, "c-wrap") {
                handle_code(el, builder);
            } else {
                // Обычный div — граница блока.
                builder.flush();
                walk_children(el, marks, builder, base);
                builder.flush();
            }
        }

        "span" => {
            if has_class(el, "post-br") {
                builder.push_break();
                return; // внутри служебный <br>
            }
            if has_class(el, "post-hr") {
                builder.push_block(ContentBlock::Hr);
                return;
            }
            let mut marks = marks.clone();
            if has_class(el, "post-b") {
                marks.bold = true;
            }
            if has_class(el, "post-i") {
                marks.italic = true;
            }
            if has_class(el, "post-u") {
                marks.underline = true;
            }
            if has_class(el, "post-s") {
                marks.strike = true;
            }
            if let Some(style) = el.value().attr("style") {
                apply_style(style, &mut marks);
            }
            walk_children(el, &marks, builder, base);
        }

        "b" | "strong" => {
            let mut marks = marks.clone();
            marks.bold = true;
            walk_children(el, &marks, builder, base);
        }
        "i" | "em" => {
            let mut marks = marks.clone();
            marks.italic = true;
            walk_children(el, &marks, builder, base);
        }
        "u" => {
            let mut marks = marks.clone();
            marks.underline = true;
            walk_children(el, &marks, builder, base);
        }
        "s" | "strike" | "del" => {
            let mut marks = marks.clone();
            marks.strike = true;
            walk_children(el, &marks, builder, base);
        }

        "pre" => {
            builder.push_block(ContentBlock::Code {
                text: text_with_breaks(el),
            });
        }

        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            builder.flush();
            let mut marks = marks.clone();
            marks.bold = true;
            walk_children(el, &marks, builder, base);
            builder.flush();
        }

        "script" | "style" | "form" | "input" => {}

        // tbody/tr/td, p и прочие контейнеры — просто проходим насквозь.
        _ => {
            if let Some(style) = el.value().attr("style") {
                let mut marks = marks.clone();
                apply_style(style, &mut marks);
                walk_children(el, &marks, builder, base);
            } else {
                walk_children(el, marks, builder, base);
            }
        }
    }
}

fn handle_link(el: ElementRef<'_>, marks: &Marks, builder: &mut Builder, base: &Url) {
    // Magnet уже вынесен в метаданные раздачи — в тексте не дублируем.
    if has_class(el, "magnet-link") {
        return;
    }
    let Some(href) = el.value().attr("href") else {
        walk_children(el, marks, builder, base);
        return;
    };
    if href.starts_with("javascript:") || href == "#" {
        walk_children(el, marks, builder, base);
        return;
    }

    let text = element_text(el);
    if text.is_empty() {
        // Ссылка-обёртка вокруг картинки: рендерим содержимое.
        walk_children(el, marks, builder, base);
        return;
    }

    let topic_id = if href.contains("viewtopic.php") {
        query_param(href, "t").and_then(|v| v.parse().ok())
    } else {
        None
    };

    builder.push_inline(Inline::Link {
        href: resolve_href(href, base),
        text,
        topic_id,
    });
}

fn handle_spoiler(el: ElementRef<'_>, marks: &Marks, builder: &mut Builder, base: &Url) {
    let title = el
        .select(&SP_HEAD)
        .next()
        .map(element_text)
        .map(|t| t.trim_end_matches(':').trim().to_owned())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "Спойлер".to_owned());

    let mut inner = Builder::default();
    if let Some(body) = el.select(&SP_BODY).next() {
        walk_children(body, marks, &mut inner, base);
    } else {
        // Нестандартный спойлер: обходим всё, кроме заголовка.
        for child in el.children() {
            if let Some(child_el) = ElementRef::wrap(child) {
                if has_class(child_el, "sp-head") {
                    continue;
                }
                handle_element(child_el, marks, &mut inner, base);
            } else if let Node::Text(text) = child.value() {
                inner.push_text(&text.text, marks);
            }
        }
    }

    builder.push_block(ContentBlock::Spoiler {
        title,
        blocks: inner.finish(),
    });
}

fn handle_quote(el: ElementRef<'_>, marks: &Marks, builder: &mut Builder, base: &Url) {
    let quote_el = el.select(&Q_INNER).next().unwrap_or(el);

    let author = el
        .select(&Q_HEAD)
        .next()
        .map(element_text)
        .map(clean_quote_author)
        .filter(|a| !a.is_empty());

    let mut inner = Builder::default();
    for child in quote_el.children() {
        if let Some(child_el) = ElementRef::wrap(child) {
            if has_class(child_el, "q-head") {
                continue;
            }
            handle_element(child_el, marks, &mut inner, base);
        } else if let Node::Text(text) = child.value() {
            inner.push_text(&text.text, marks);
        }
    }

    builder.push_block(ContentBlock::Quote {
        author,
        blocks: inner.finish(),
    });
}

fn clean_quote_author(head: String) -> String {
    head.replace("писал(а):", "")
        .replace("писал(а)", "")
        .replace("Цитата:", "")
        .replace("Цитата", "")
        .trim()
        .to_owned()
}

fn handle_code(el: ElementRef<'_>, builder: &mut Builder) {
    let text = el
        .select(&C_BODY)
        .next()
        .map(text_with_breaks)
        .unwrap_or_else(|| text_with_breaks(el));
    builder.push_block(ContentBlock::Code { text });
}

/// Текст с сохранением переносов (`<br>` → `\n`), для кода.
fn text_with_breaks(el: ElementRef<'_>) -> String {
    let mut out = String::new();
    collect_text_with_breaks(el, &mut out);
    out.trim_matches('\n').trim_end().to_owned()
}

fn collect_text_with_breaks(el: ElementRef<'_>, out: &mut String) {
    for child in el.children() {
        match child.value() {
            Node::Text(text) => out.push_str(&text.text),
            Node::Element(element) => {
                if element.name() == "br" {
                    out.push('\n');
                } else if let Some(child_el) = ElementRef::wrap(child) {
                    collect_text_with_breaks(child_el, out);
                }
            }
            _ => {}
        }
    }
}

fn apply_style(style: &str, marks: &mut Marks) {
    for decl in style.split(';') {
        let Some((prop, value)) = decl.split_once(':') else {
            continue;
        };
        let prop = prop.trim().to_ascii_lowercase();
        let value = value.trim().to_ascii_lowercase();
        match prop.as_str() {
            "color" => {
                let safe: String = value
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '#' | '(' | ')' | ',' | '.' | '%' | ' ' | '-'))
                    .collect();
                if !safe.is_empty() {
                    marks.color = Some(safe.trim().to_owned());
                }
            }
            "font-weight" => {
                if value.contains("bold") || value.parse::<u32>().map(|w| w >= 600).unwrap_or(false)
                {
                    marks.bold = true;
                }
            }
            "font-style" => {
                if value.contains("italic") {
                    marks.italic = true;
                }
            }
            "text-decoration" | "text-decoration-line" => {
                if value.contains("underline") {
                    marks.underline = true;
                }
                if value.contains("line-through") {
                    marks.strike = true;
                }
            }
            _ => {}
        }
    }
}

fn resolve_href(href: &str, base: &Url) -> String {
    match Url::parse(href) {
        Ok(url) => url.to_string(),
        Err(_) => base
            .join(href)
            .map(|u| u.to_string())
            .unwrap_or_else(|_| href.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use scraper::Html;

    use super::*;

    fn parse_blocks(body_html: &str) -> Vec<ContentBlock> {
        let html = format!(r#"<html><body><div class="post_body">{body_html}</div></body></html>"#);
        let doc = Html::parse_document(&html);
        let sel = Selector::parse("div.post_body").unwrap();
        let root = doc.select(&sel).next().unwrap();
        let base = Url::parse("https://rutracker.org/forum/").unwrap();
        blocks_from_post_body(root, &base)
    }

    #[test]
    fn plain_text_with_styles() {
        let blocks = parse_blocks(
            r#"Обычный <span class="post-b">жирный</span> и <span class="post-i">курсив</span>"#,
        );
        assert_eq!(blocks.len(), 1);
        let ContentBlock::Paragraph { inlines } = &blocks[0] else {
            panic!("ожидался параграф");
        };
        assert_eq!(inlines.len(), 4);
        assert!(matches!(&inlines[1], Inline::Text { text, bold: true, .. } if text == "жирный"));
        assert!(matches!(&inlines[3], Inline::Text { text, italic: true, .. } if text == "курсив"));
    }

    #[test]
    fn breaks_and_paragraph_boundaries() {
        let blocks = parse_blocks(
            r#"Первая строка<span class="post-br"><br></span>Вторая строка<div class="post-box">Отдельный блок</div>"#,
        );
        assert_eq!(blocks.len(), 2);
        let ContentBlock::Paragraph { inlines } = &blocks[0] else {
            panic!()
        };
        assert!(matches!(inlines[1], Inline::Break));
        assert!(matches!(&blocks[1], ContentBlock::Paragraph { inlines } if inlines.len() == 1));
    }

    #[test]
    fn spoiler_with_image_and_title() {
        let blocks = parse_blocks(
            r#"<div class="sp-wrap"><div class="sp-head folded"><span>Скриншоты</span></div>
               <div class="sp-body"><var class="postImg" title="https://i.example/1.png">картинка</var></div></div>"#,
        );
        assert_eq!(blocks.len(), 1);
        let ContentBlock::Spoiler { title, blocks } = &blocks[0] else {
            panic!("ожидался спойлер");
        };
        assert_eq!(title, "Скриншоты");
        assert_eq!(
            blocks[0],
            ContentBlock::Image {
                src: "https://i.example/1.png".to_owned()
            }
        );
    }

    #[test]
    fn quote_with_author() {
        let blocks = parse_blocks(
            r#"<div class="q-wrap"><div class="q"><div class="q-head"><a>user42 писал(а):</a></div>Текст цитаты</div></div>"#,
        );
        let ContentBlock::Quote { author, blocks } = &blocks[0] else {
            panic!("ожидалась цитата");
        };
        assert_eq!(author.as_deref(), Some("user42"));
        assert!(matches!(&blocks[0], ContentBlock::Paragraph { .. }));
    }

    #[test]
    fn code_block_preserves_lines() {
        let blocks = parse_blocks(
            r#"<div class="c-wrap"><div class="c-head"><b>Код:</b></div><div class="c-body">line1<br>line2</div></div>"#,
        );
        assert_eq!(
            blocks[0],
            ContentBlock::Code {
                text: "line1\nline2".to_owned()
            }
        );
    }

    #[test]
    fn lists_and_links() {
        let blocks = parse_blocks(
            r#"<ul><li>Первый</li><li><a href="viewtopic.php?t=555">Раздача</a></li></ul>"#,
        );
        let ContentBlock::List { ordered, items } = &blocks[0] else {
            panic!("ожидался список");
        };
        assert!(!ordered);
        assert_eq!(items.len(), 2);
        let ContentBlock::Paragraph { inlines } = &items[1][0] else {
            panic!()
        };
        let Inline::Link { topic_id, href, .. } = &inlines[0] else {
            panic!("ожидалась ссылка")
        };
        assert_eq!(*topic_id, Some(555));
        assert!(href.starts_with("https://rutracker.org/forum/viewtopic.php"));
    }

    #[test]
    fn magnet_links_are_skipped_in_body() {
        let html = r#"Текст <a class="magnet-link" href="magnet:?xt=urn:btih:abc">m</a> дальше"#;
        let blocks = parse_blocks(html);
        let ContentBlock::Paragraph { inlines } = &blocks[0] else {
            panic!()
        };
        assert_eq!(inlines.len(), 1);
        assert!(matches!(&inlines[0], Inline::Text { text, .. } if text == "Текст дальше"));
    }
}

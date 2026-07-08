//! Разбор дерева форумов из `<select id="fs-main">` на странице поиска.

use std::sync::LazyLock;

use scraper::{Html, Selector};

use crate::{
    Result,
    error::Error,
    models::{ForumEntry, ForumGroup},
    parse::common::collapse_whitespace,
};

static SELECT: LazyLock<Selector> = LazyLock::new(|| Selector::parse("#fs-main").expect("selector"));
static OPTGROUP: LazyLock<Selector> = LazyLock::new(|| Selector::parse("optgroup").expect("selector"));
static OPTION: LazyLock<Selector> = LazyLock::new(|| Selector::parse("option").expect("selector"));

/// Разбирает дерево «категория → форумы (с вложенностью)».
pub fn parse_forum_select(html: &str) -> Result<Vec<ForumGroup>> {
    let doc = Html::parse_document(html);
    let select = doc
        .select(&SELECT)
        .next()
        .ok_or_else(|| Error::parse("список форумов (#fs-main) не найден"))?;

    let mut groups = Vec::new();
    for group_el in select.select(&OPTGROUP) {
        let title = group_el
            .value()
            .attr("label")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("Прочее")
            .to_owned();

        let mut forums = Vec::new();
        for option in group_el.select(&OPTION) {
            let Some(value) = option.value().attr("value") else {
                continue;
            };
            let Ok(id) = value.trim().parse::<i64>() else {
                continue; // служебные пункты
            };
            let raw = option.text().collect::<String>();
            let (name, depth) = strip_indent(&raw);
            if name.is_empty() {
                continue;
            }
            forums.push(ForumEntry { id, name, depth });
        }

        if !forums.is_empty() {
            groups.push(ForumGroup { title, forums });
        }
    }

    if groups.is_empty() {
        return Err(Error::parse("дерево форумов пусто — вероятно, изменилась разметка"));
    }
    Ok(groups)
}

/// Убирает маркеры вложенности вида `|- ` и неразрывные пробелы,
/// возвращая имя и глубину.
fn strip_indent(raw: &str) -> (String, u8) {
    let mut rest = raw.trim_start_matches([' ', '\u{a0}']);
    let mut depth: u8 = 0;
    while let Some(stripped) = rest.strip_prefix("|-") {
        depth = depth.saturating_add(1);
        rest = stripped.trim_start_matches([' ', '\u{a0}']);
    }
    (collapse_whitespace(rest), depth)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HTML: &str = r#"<html><body>
      <select id="fs-main" name="f[]" multiple="multiple">
        <optgroup label="Операционные системы">
          <option value="2093">Linux-дистрибутивы</option>
          <option value="1379">&nbsp;&nbsp;|- Debian, Ubuntu</option>
          <option value="1381">&nbsp;&nbsp;|- |- Нишевые сборки</option>
        </optgroup>
        <optgroup label="Служебное">
          <option value="not-a-number">игнорировать</option>
        </optgroup>
        <optgroup label="Игры">
          <option value="50">Игры для ПК</option>
        </optgroup>
      </select>
    </body></html>"#;

    #[test]
    fn parses_groups_and_depth() {
        let groups = parse_forum_select(HTML).unwrap();
        assert_eq!(groups.len(), 2);

        let os = &groups[0];
        assert_eq!(os.title, "Операционные системы");
        assert_eq!(os.forums.len(), 3);
        assert_eq!(os.forums[0].id, 2093);
        assert_eq!(os.forums[0].depth, 0);
        assert_eq!(os.forums[1].name, "Debian, Ubuntu");
        assert_eq!(os.forums[1].depth, 1);
        assert_eq!(os.forums[2].depth, 2);

        assert_eq!(groups[1].forums[0].id, 50);
    }

    #[test]
    fn missing_select_is_error() {
        assert!(parse_forum_select("<html><body></body></html>").is_err());
    }
}

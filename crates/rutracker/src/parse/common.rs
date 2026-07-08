//! Общие утилиты парсинга.

use scraper::ElementRef;

/// Извлекает из текста первое целое число, игнорируя пробелы и разделители
/// тысяч (`1,234`, `1 234`).
pub fn first_int(text: &str) -> Option<u64> {
    let mut digits = String::new();
    let mut seen_digit = false;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            seen_digit = true;
        } else if seen_digit && !matches!(ch, ',' | ' ' | '\u{a0}' | '.') {
            break;
        } else if seen_digit && ch == '.' {
            // «1.234» считаем разделителем только если дальше снова цифры —
            // упрощённо обрываем: целые счётчики rutracker точек не содержат.
            break;
        }
    }
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

/// Разбирает человекочитаемый размер («1.37 GB», «700 МБ») в байты.
pub fn parse_size_text(text: &str) -> Option<u64> {
    let cleaned = text.replace('\u{a0}', " ");
    let mut number = String::new();
    let mut unit = String::new();
    for ch in cleaned.chars() {
        if ch.is_ascii_digit() || ch == '.' || ch == ',' {
            if !unit.is_empty() {
                break;
            }
            number.push(if ch == ',' { '.' } else { ch });
        } else if ch.is_alphabetic() {
            unit.push(ch);
        } else if !number.is_empty() && !unit.is_empty() {
            break;
        }
    }
    let value: f64 = number.parse().ok()?;
    let multiplier: f64 = match unit.to_lowercase().as_str() {
        "b" | "б" => 1.0,
        "kb" | "кб" => 1024.0,
        "mb" | "мб" => 1024.0 * 1024.0,
        "gb" | "гб" => 1024.0 * 1024.0 * 1024.0,
        "tb" | "тб" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    let bytes = value * multiplier;
    if bytes.is_finite() && bytes >= 0.0 {
        Some(bytes.round() as u64)
    } else {
        None
    }
}

/// Текст элемента: конкатенация текстовых узлов со схлопнутыми пробелами.
pub fn element_text(el: ElementRef<'_>) -> String {
    collapse_whitespace(&el.text().collect::<String>())
}

/// Схлопывает последовательности пробельных символов в один пробел.
pub fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_space = true;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out.trim_end().to_owned()
}

/// Значение параметра из query-строки ссылки (`viewtopic.php?t=123` → `123`).
pub fn query_param(href: &str, name: &str) -> Option<String> {
    let query = href.split_once('?')?.1;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=')?;
        if key == name {
            return Some(value.split('#').next().unwrap_or(value).to_owned());
        }
    }
    None
}

/// Есть ли у элемента данный CSS-класс.
pub fn has_class(el: ElementRef<'_>, class: &str) -> bool {
    el.value().classes().any(|c| c == class)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ints_with_separators() {
        assert_eq!(first_int("2,404"), Some(2404));
        assert_eq!(first_int(" 1 234 раз"), Some(1234));
        assert_eq!(first_int("нет цифр"), None);
    }

    #[test]
    fn parses_sizes() {
        assert_eq!(parse_size_text("700 MB"), Some(700 * 1024 * 1024));
        assert_eq!(parse_size_text("1.5 ГБ"), Some(1_610_612_736));
        assert_eq!(parse_size_text("1,37\u{a0}GB"), Some(1_471_026_299));
        assert_eq!(parse_size_text("мусор"), None);
    }

    #[test]
    fn extracts_query_params() {
        assert_eq!(
            query_param("viewtopic.php?t=6335144", "t").as_deref(),
            Some("6335144")
        );
        assert_eq!(
            query_param("tracker.php?f=50&nm=test", "nm").as_deref(),
            Some("test")
        );
        assert_eq!(query_param("index.php", "t"), None);
    }

    #[test]
    fn collapses_whitespace() {
        assert_eq!(collapse_whitespace("  a\n\t b  c "), "a b c");
    }
}

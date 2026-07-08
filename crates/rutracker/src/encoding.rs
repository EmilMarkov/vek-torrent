//! Работа с кодировкой windows-1251: декодирование ответов и кодирование форм.

use encoding_rs::{Encoding, WINDOWS_1251};

/// Декодирует тело ответа rutracker в UTF-8-строку.
///
/// Кодировка берётся из заголовка `Content-Type`; по умолчанию — windows-1251,
/// так как rutracker отдаёт страницы именно в ней.
pub fn decode_body(bytes: &[u8], content_type: Option<&str>) -> String {
    let encoding = content_type
        .and_then(charset_from_content_type)
        .unwrap_or(WINDOWS_1251);
    let (text, _, _) = encoding.decode(bytes);
    text.into_owned()
}

fn charset_from_content_type(content_type: &str) -> Option<&'static Encoding> {
    for part in content_type.split(';') {
        let part = part.trim();
        if let Some(prefix) = part.get(..8)
            && prefix.eq_ignore_ascii_case("charset=")
        {
            let label = part[8..].trim_matches(|c| c == '"' || c == '\'');
            return Encoding::for_label(label.as_bytes());
        }
    }
    None
}

/// Кодирует текст в windows-1251 (полезно в тестах и при сборке форм).
pub fn encode_cp1251(text: &str) -> Vec<u8> {
    WINDOWS_1251.encode(text).0.into_owned()
}

/// Кодирует пары `ключ=значение` в `application/x-www-form-urlencoded`,
/// предварительно переведя значения в windows-1251 — иначе rutracker
/// не поймёт кириллицу в формах (логин, поисковый запрос).
pub fn cp1251_form_urlencode<K, V>(pairs: &[(K, V)]) -> String
where
    K: AsRef<str>,
    V: AsRef<str>,
{
    pairs
        .iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                encode_component(key.as_ref()),
                encode_component(value.as_ref())
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

fn encode_component(text: &str) -> String {
    let (bytes, _, _) = WINDOWS_1251.encode(text);
    let mut out = String::with_capacity(bytes.len() * 3);
    for &byte in bytes.iter() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'*' => {
                out.push(byte as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_cyrillic_as_cp1251_percents() {
        // "Вход" в windows-1251: C2 F5 EE E4
        assert_eq!(encode_component("Вход"), "%C2%F5%EE%E4");
    }

    #[test]
    fn encodes_spaces_and_safe_chars() {
        assert_eq!(encode_component("linux mint 21.3"), "linux+mint+21.3");
    }

    #[test]
    fn builds_form_body() {
        let body = cp1251_form_urlencode(&[("nm", "тест"), ("o", "10")]);
        assert_eq!(body, "nm=%F2%E5%F1%F2&o=10");
    }

    #[test]
    fn decodes_cp1251_by_default() {
        // "Привет" в windows-1251
        let bytes = [0xCF, 0xF0, 0xE8, 0xE2, 0xE5, 0xF2];
        assert_eq!(decode_body(&bytes, None), "Привет");
    }

    #[test]
    fn respects_charset_from_header() {
        let bytes = "Привет".as_bytes();
        assert_eq!(
            decode_body(bytes, Some("text/html; charset=UTF-8")),
            "Привет"
        );
    }
}

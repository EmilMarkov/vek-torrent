//! Разбор страницы логина: детект формы, капчи и причин отказа.

use std::sync::LazyLock;

use scraper::{Html, Selector};
use url::Url;

use crate::{
    error::Error,
    models::{CaptchaChallenge, SessionInfo},
};

static LOGIN_INPUT: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"input[name="login_username"]"#).expect("selector"));
static LOGGED_USERNAME: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("#logged-in-username").expect("selector"));
static CAPTCHA_IMG: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"img[src*="captcha"]"#).expect("selector"));
static CAPTCHA_SID: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"input[name="cap_sid"]"#).expect("selector"));
static CAPTCHA_CODE: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"input[name^="cap_code_"]"#).expect("selector"));

/// Похожа ли страница на форму логина (значит, сессии нет).
pub fn is_login_page(html: &str) -> bool {
    let doc = Html::parse_document(html);
    doc.select(&LOGIN_INPUT).next().is_some() && doc.select(&LOGGED_USERNAME).next().is_none()
}

/// Похожа ли страница на rutracker вообще (форма логина или имя вошедшего).
///
/// Используется для проверки зеркал: заглушка провайдера при блокировке
/// не содержит ни того, ни другого.
pub fn looks_like_rutracker(html: &str) -> bool {
    let doc = Html::parse_document(html);
    doc.select(&LOGIN_INPUT).next().is_some() || doc.select(&LOGGED_USERNAME).next().is_some()
}

/// Состояние сессии по любой странице форума.
pub fn session_info(html: &str) -> SessionInfo {
    let doc = Html::parse_document(html);
    match doc.select(&LOGGED_USERNAME).next() {
        Some(el) => SessionInfo {
            logged_in: true,
            username: Some(super::common::element_text(el)).filter(|s| !s.is_empty()),
        },
        None => SessionInfo {
            logged_in: false,
            username: None,
        },
    }
}

/// Классифицирует неудачный логин по HTML-ответу.
pub fn classify_failure(html: &str, base: &Url) -> Error {
    let doc = Html::parse_document(html);

    if let Some(challenge) = extract_captcha(&doc, base) {
        return Error::CaptchaRequired(Box::new(challenge));
    }

    let text = doc.root_element().text().collect::<String>().to_lowercase();
    if text.contains("неверн") && (text.contains("парол") || text.contains("имя")) {
        return Error::BadCredentials;
    }
    if text.contains("заблокирован") || text.contains("отключен") {
        return Error::AccessDenied("аккаунт заблокирован или отключён".into());
    }

    Error::Parse("не удалось выполнить вход: трекер вернул неожиданную страницу".into())
}

fn extract_captcha(doc: &Html, base: &Url) -> Option<CaptchaChallenge> {
    let sid = doc.select(&CAPTCHA_SID).next()?.value().attr("value")?;
    let code_field = doc.select(&CAPTCHA_CODE).next()?.value().attr("name")?;
    let img_src = doc.select(&CAPTCHA_IMG).next()?.value().attr("src")?;
    let img_url = Url::parse(img_src)
        .or_else(|_| base.join(img_src))
        .ok()?
        .to_string();
    Some(CaptchaChallenge {
        sid: sid.to_owned(),
        code_field: code_field.to_owned(),
        img_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOGIN_PAGE: &str = r#"<html><body>
      <form action="login.php" method="post">
        <input type="text" name="login_username" />
        <input type="password" name="login_password" />
      </form></body></html>"#;

    const CAPTCHA_PAGE: &str = r#"<html><body>
      <form action="login.php" method="post">
        <input type="text" name="login_username" />
        <img src="https://static.rutracker.cc/captcha/1/abc.jpg" />
        <input type="hidden" name="cap_sid" value="SID123" />
        <input type="text" name="cap_code_deadbeef" />
      </form></body></html>"#;

    fn base() -> Url {
        Url::parse("https://rutracker.org/forum/").unwrap()
    }

    #[test]
    fn detects_login_page() {
        let logged_in = r#"<html><body><div id="logged-in-username">user</div></body></html>"#;
        assert!(is_login_page(LOGIN_PAGE));
        assert!(!is_login_page(logged_in));
    }

    #[test]
    fn extracts_captcha_challenge() {
        match classify_failure(CAPTCHA_PAGE, &base()) {
            Error::CaptchaRequired(challenge) => {
                assert_eq!(challenge.sid, "SID123");
                assert_eq!(challenge.code_field, "cap_code_deadbeef");
                assert!(challenge.img_url.contains("captcha"));
            }
            other => panic!("ожидалась капча, получено: {other:?}"),
        }
    }

    #[test]
    fn classifies_bad_credentials() {
        let html = r#"<html><body><h4 class="warnColor1">Вы ввели неверное имя пользователя или пароль</h4>
          <input name="login_username" /></body></html>"#;
        assert!(matches!(
            classify_failure(html, &base()),
            Error::BadCredentials
        ));
    }

    #[test]
    fn reads_session_username() {
        let html = r#"<html><body><a id="logged-in-username">torrent_fan</a></body></html>"#;
        let info = session_info(html);
        assert!(info.logged_in);
        assert_eq!(info.username.as_deref(), Some("torrent_fan"));
    }
}

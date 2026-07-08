//! Интеграционные тесты клиента rutracker против wiremock-сервера:
//! логин (успех/капча/ошибки), кодировка форм, поиск, пагинация,
//! страница раздачи, скачивание `.torrent`, персистентность куков.

use std::time::Duration;

use rutracker::{
    Client, Error,
    encoding::encode_cp1251,
    models::{SearchRequest, SortField, SortOrder},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_string_contains, method, path, query_param},
};

const SEARCH_FIXTURE: &str = include_str!("fixtures/search_results.html");
const TOPIC_FIXTURE: &str = include_str!("fixtures/topic_page.html");

const LOGGED_IN_PAGE: &str = r#"<html><body>
  <a id="logged-in-username" href="profile.php?mode=viewprofile">tester</a>
</body></html>"#;

const LOGIN_PAGE: &str = r#"<html><body>
  <form action="login.php" method="post">
    <input type="text" name="login_username" />
    <input type="password" name="login_password" />
  </form>
</body></html>"#;

const CAPTCHA_PAGE: &str = r#"<html><body>
  <form action="login.php" method="post">
    <input type="text" name="login_username" />
    <img src="https://static.rutracker.cc/captcha/1/2/abc123.jpg" />
    <input type="hidden" name="cap_sid" value="SID123" />
    <input type="text" name="cap_code_deadbeef" />
  </form>
</body></html>"#;

const CATEGORIES_PAGE: &str = r#"<html><body>
  <select id="fs-main" name="f[]" multiple="multiple">
    <optgroup label="Операционные системы">
      <option value="2093">Linux-дистрибутивы</option>
      <option value="1379">&nbsp;&nbsp;|- Debian, Ubuntu</option>
    </optgroup>
  </select>
</body></html>"#;

fn html_cp1251(body: &str) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_raw(encode_cp1251(body), "text/html; charset=windows-1251")
}

fn client_for(server: &MockServer) -> Client {
    Client::builder()
        .base_url(server.uri())
        .timeout(Duration::from_secs(5))
        .build()
        .expect("клиент должен собираться")
}

#[tokio::test]
async fn login_success_sets_session_cookie() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/forum/login.php"))
        .and(body_string_contains("login_username=user"))
        // «Вход» обязан уйти в windows-1251: C2 F5 EE E4
        .and(body_string_contains("login=%C2%F5%EE%E4"))
        .respond_with(
            html_cp1251(LOGGED_IN_PAGE)
                .insert_header("set-cookie", "bb_session=1-abcdef; Path=/; Max-Age=31536000"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server);
    assert!(!client.has_session_cookie());
    client.login("user", "pass", None).await.expect("вход должен пройти");
    assert!(client.has_session_cookie());
}

#[tokio::test]
async fn login_reports_captcha_challenge() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/forum/login.php"))
        .respond_with(html_cp1251(CAPTCHA_PAGE))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .login("user", "pass", None)
        .await
        .expect_err("должна потребоваться капча");

    match err {
        Error::CaptchaRequired(challenge) => {
            assert_eq!(challenge.sid, "SID123");
            assert_eq!(challenge.code_field, "cap_code_deadbeef");
            assert!(challenge.img_url.contains("captcha"));
        }
        other => panic!("ожидалась капча, получено: {other:?}"),
    }
}

#[tokio::test]
async fn login_reports_bad_credentials() {
    let page = r#"<html><body>
      <h4 class="warnColor1">Вы ввели неверное имя пользователя или пароль</h4>
      <form><input name="login_username" /></form>
    </body></html>"#;

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/forum/login.php"))
        .respond_with(html_cp1251(page))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .login("user", "wrong", None)
        .await
        .expect_err("вход не должен пройти");
    assert!(matches!(err, Error::BadCredentials));
}

#[tokio::test]
async fn search_sends_cp1251_form_and_parses_results() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/forum/tracker.php"))
        // «тест» в windows-1251: F2 E5 F1 F2
        .and(body_string_contains("nm=%F2%E5%F1%F2"))
        .and(body_string_contains("o=10"))
        .and(body_string_contains("s=2"))
        .and(body_string_contains("f%5B%5D=1379"))
        .respond_with(html_cp1251(SEARCH_FIXTURE))
        .expect(1)
        .mount(&server)
        .await;

    let request = SearchRequest {
        query: "тест".to_owned(),
        forums: vec![1379],
        sort: SortField::Seeders,
        order: SortOrder::Desc,
        ..Default::default()
    };
    let page = client_for(&server).search(&request).await.expect("поиск должен пройти");

    assert_eq!(page.items.len(), 2);
    assert_eq!(page.total_found, 2);
    assert!(page.items[0].title.contains("Linux Mint"));
    assert_eq!(page.search_id.as_deref(), Some("Af1B2c3D"));
    assert!(!page.has_more());
}

#[tokio::test]
async fn search_pagination_uses_search_id() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/forum/tracker.php"))
        .and(query_param("search_id", "Af1B2c3D"))
        .and(query_param("start", "50"))
        .respond_with(html_cp1251(SEARCH_FIXTURE))
        .expect(1)
        .mount(&server)
        .await;

    let request = SearchRequest {
        query: "тест".to_owned(),
        offset: 50,
        search_id: Some("Af1B2c3D".to_owned()),
        ..Default::default()
    };
    let page = client_for(&server).search(&request).await.expect("пагинация должна работать");
    assert_eq!(page.offset, 50);
}

#[tokio::test]
async fn search_without_session_is_not_authenticated() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/forum/tracker.php"))
        .respond_with(html_cp1251(LOGIN_PAGE))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .search(&SearchRequest {
            query: "тест".to_owned(),
            ..Default::default()
        })
        .await
        .expect_err("без сессии поиск недоступен");
    assert!(matches!(err, Error::NotAuthenticated));
}

#[tokio::test]
async fn topic_page_is_parsed() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/forum/viewtopic.php"))
        .and(query_param("t", "6335144"))
        .respond_with(html_cp1251(TOPIC_FIXTURE))
        .mount(&server)
        .await;

    let topic = client_for(&server).topic(6335144).await.expect("раздача должна разобраться");
    assert!(topic.title.contains("Linux Mint"));
    assert!(topic.magnet.is_some());
    assert!(topic.has_torrent_file);
    assert!(!topic.body.is_empty());
}

#[tokio::test]
async fn categories_are_parsed() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/forum/tracker.php"))
        .respond_with(html_cp1251(CATEGORIES_PAGE))
        .mount(&server)
        .await;

    let groups = client_for(&server).categories().await.expect("дерево должно разобраться");
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].forums.len(), 2);
    assert_eq!(groups[0].forums[1].depth, 1);
}

#[tokio::test]
async fn download_torrent_returns_bytes_and_filename() {
    let torrent_bytes = b"d8:announce26:http://bt.example.host/ann4:infod4:name4:teste".to_vec();

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/forum/dl.php"))
        .and(query_param("t", "6335144"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(torrent_bytes.clone(), "application/x-bittorrent")
                .insert_header(
                    "content-disposition",
                    r#"attachment; filename="[rutracker.org].t6335144.torrent""#,
                ),
        )
        .mount(&server)
        .await;

    let torrent = client_for(&server)
        .download_torrent(6335144)
        .await
        .expect(".torrent должен скачаться");
    assert_eq!(torrent.bytes, torrent_bytes);
    assert_eq!(
        torrent.filename.as_deref(),
        Some("[rutracker.org].t6335144.torrent")
    );
}

#[tokio::test]
async fn download_torrent_detects_logged_out_session() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/forum/dl.php"))
        .respond_with(html_cp1251(LOGIN_PAGE))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .download_torrent(6335144)
        .await
        .expect_err("HTML вместо торрента — это ошибка");
    assert!(matches!(err, Error::NotAuthenticated));
}

#[tokio::test]
async fn cookies_persist_between_clients() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cookie_path = dir.path().join("cookies.json");

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/forum/login.php"))
        .respond_with(
            html_cp1251(LOGGED_IN_PAGE)
                .insert_header("set-cookie", "bb_session=1-abcdef; Path=/; Max-Age=31536000"),
        )
        .mount(&server)
        .await;

    {
        let client = Client::builder()
            .base_url(server.uri())
            .cookie_path(Some(cookie_path.clone()))
            .build()
            .unwrap();
        client.login("user", "pass", None).await.unwrap();
        assert!(client.has_session_cookie());
    }

    let restored = Client::builder()
        .base_url(server.uri())
        .cookie_path(Some(cookie_path))
        .build()
        .unwrap();
    assert!(
        restored.has_session_cookie(),
        "сессия должна восстанавливаться из файла куков"
    );

    // Локальный «выход» стирает сессию.
    restored.logout().unwrap();
    assert!(!restored.has_session_cookie());
}

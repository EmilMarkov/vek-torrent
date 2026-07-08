//! Интеграционные тесты клиента qBittorrent против wiremock.

use std::time::Duration;

use qbit::{
    Client, ClientConfig, Error,
    models::{AddTorrent, TorrentFilter, TorrentSource, TorrentState, TorrentsQuery},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_string_contains, header, method, path, query_param},
};

fn client_for(server: &MockServer) -> Client {
    Client::new(ClientConfig {
        base_url: server.uri(),
        timeout: Duration::from_secs(5),
    })
    .expect("клиент qBittorrent")
}

#[tokio::test]
async fn login_ok() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .and(body_string_contains("username=admin"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Ok."))
        .mount(&server)
        .await;

    client_for(&server).login("admin", "secret").await.expect("вход");
}

#[tokio::test]
async fn login_wrong_password() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Fails."))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .login("admin", "bad")
        .await
        .expect_err("должно провалиться");
    assert!(matches!(err, Error::AuthFailed));
}

#[tokio::test]
async fn login_banned_on_forbidden() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;

    assert!(matches!(
        client_for(&server).login("a", "b").await,
        Err(Error::Banned)
    ));
}

#[tokio::test]
async fn version_and_health() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/app/webapiVersion"))
        .respond_with(ResponseTemplate::new(200).set_body_string("2.11.2"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v2/app/version"))
        .respond_with(ResponseTemplate::new(200).set_body_string("v5.0.3\n"))
        .mount(&server)
        .await;

    let client = client_for(&server);
    assert!(client.is_alive().await);
    assert_eq!(client.version().await.unwrap(), "v5.0.3");
    assert_eq!(client.web_api_version().await.unwrap(), "2.11.2");
}

#[tokio::test]
async fn torrents_info_is_parsed() {
    let body = r#"[
      {"hash":"abc123","name":"Ubuntu ISO","size":734003200,"progress":0.5,
       "dlspeed":1048576,"upspeed":0,"eta":3600,"state":"downloading",
       "category":"linux","tags":"","save_path":"/downloads","num_seeds":10,
       "num_leechs":2,"ratio":0.1,"completed":367001600,"amount_left":367001600,
       "added_on":1726000000,"completion_on":-1},
      {"hash":"def456","name":"Movie","size":100,"progress":1.0,"state":"stoppedUP"}
    ]"#;

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .and(query_param("filter", "downloading"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;

    let list = client_for(&server)
        .torrents(&TorrentsQuery {
            filter: TorrentFilter::Downloading,
            ..Default::default()
        })
        .await
        .expect("список торрентов");

    assert_eq!(list.len(), 2);
    assert_eq!(list[0].name, "Ubuntu ISO");
    assert_eq!(list[0].state_kind(), TorrentState::Downloading);
    assert!(list[0].state_kind().is_active());
    // stoppedUP (v5) должно нормализоваться в Paused.
    assert_eq!(list[1].state_kind(), TorrentState::Paused);
}

#[tokio::test]
async fn transfer_info_is_parsed() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/transfer/info"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"dl_info_speed":2048,"up_info_speed":512,"connection_status":"connected"}"#,
        ))
        .mount(&server)
        .await;

    let info = client_for(&server).transfer_info().await.unwrap();
    assert_eq!(info.dl_info_speed, 2048);
    assert_eq!(info.connection_status, "connected");
}

#[tokio::test]
async fn add_torrent_by_url_sends_multipart() {
    let server = MockServer::start().await;
    // Клиент шлёт Referer с завершающим слэшем (base.as_str()).
    let referer = format!("{}/", server.uri());
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/add"))
        .and(header("referer", referer.as_str()))
        .and(body_string_contains("magnet:?xt=urn:btih:DEADBEEF"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Ok."))
        .mount(&server)
        .await;

    client_for(&server)
        .add_torrent(&AddTorrent {
            sources: vec![TorrentSource::Url("magnet:?xt=urn:btih:DEADBEEF".to_owned())],
            category: Some("linux".to_owned()),
            ..Default::default()
        })
        .await
        .expect("добавление magnet");
}

#[tokio::test]
async fn add_torrent_rejects_on_fails() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/add"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Fails."))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .add_torrent(&AddTorrent {
            sources: vec![TorrentSource::File {
                filename: "bad.torrent".to_owned(),
                bytes: vec![1, 2, 3],
            }],
            ..Default::default()
        })
        .await
        .expect_err("повреждённый торрент");
    assert!(matches!(err, Error::OperationFailed(_)));
}

#[tokio::test]
async fn stop_uses_v5_endpoint_when_available() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/stop"))
        .and(body_string_contains("hashes=abc"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    client_for(&server).stop(&["abc".to_owned()]).await.expect("stop v5");
}

#[tokio::test]
async fn stop_falls_back_to_v4_pause() {
    let server = MockServer::start().await;
    // v5-метод отсутствует.
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/stop"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    // Клиент должен повторить со старым именем.
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/pause"))
        .and(body_string_contains("hashes=abc"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    client_for(&server)
        .stop(&["abc".to_owned()])
        .await
        .expect("должен сработать откат на pause");
}

#[tokio::test]
async fn categories_normalizes_names() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/categories"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"linux":{"name":"linux","savePath":"/d/linux"},"films":{"name":"","savePath":"/d/films"}}"#,
        ))
        .mount(&server)
        .await;

    let cats = client_for(&server).categories().await.unwrap();
    assert_eq!(cats.len(), 2);
    // Отсортированы по имени; пустое имя заменяется ключом.
    assert_eq!(cats[0].name, "films");
    assert_eq!(cats[1].name, "linux");
}

#[tokio::test]
async fn delete_sends_hashes_and_flag() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/delete"))
        .and(body_string_contains("hashes=a%7Cb"))
        .and(body_string_contains("deleteFiles=true"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    client_for(&server)
        .delete(&["a".to_owned(), "b".to_owned()], true)
        .await
        .expect("удаление");
}

//! Интеграционные тесты API-слоя: маршрутизация, авторизация, OpenAPI.
//!
//! Проверяем то, что не требует сети: health без токена, отказ без/с неверным
//! токеном (middleware отклоняет до обращения к ядру), выдачу OpenAPI-спеки.

use std::sync::Arc;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use vek_api::{ApiState, build_app};
use vek_core::AppCore;

const TOKEN: &str = "test-token-1234567890";

fn app() -> axum::Router {
    let dir = tempfile::tempdir().unwrap().keep();
    let core = AppCore::new(dir).unwrap();
    build_app(ApiState {
        core,
        token: Arc::from(TOKEN),
    })
}

async fn body_string(response: axum::response::Response) -> String {
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn health_is_public() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let text = body_string(response).await;
    assert!(text.contains("\"status\":\"ok\""));
    assert!(text.contains("VEK Torrent"));
}

#[tokio::test]
async fn protected_route_without_token_is_401() {
    let response = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/search")
                .header("content-type", "application/json")
                .body(Body::from("{\"query\":\"x\"}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let text = body_string(response).await;
    assert!(text.contains("\"code\":\"unauthorized\""));
}

#[tokio::test]
async fn protected_route_with_wrong_token_is_401() {
    let response = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/search")
                .header("authorization", "Bearer wrong-token")
                .header("content-type", "application/json")
                .body(Body::from("{\"query\":\"x\"}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn openapi_spec_is_served() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let text = body_string(response).await;
    assert!(text.contains("VEK Torrent API"));
    assert!(text.contains("/api/v1/search"));
    assert!(text.contains("/api/v1/downloads"));
}

#[tokio::test]
async fn unknown_route_is_404() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/does-not-exist")
                .header("authorization", format!("Bearer {TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

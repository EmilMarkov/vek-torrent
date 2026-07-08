//! Внешний REST API VEK Torrent на axum с документацией OpenAPI (Swagger UI).
//!
//! Все содержательные операции делегируются в [`vek_core::AppCore`]. Доступ к
//! `/api/v1/*` (кроме `/health`) защищён Bearer-токеном; интерактивная
//! документация доступна на `/docs`, спецификация — на `/api-docs/openapi.json`.

mod error;
mod routes;

use std::{net::SocketAddr, sync::Arc};

use axum::{
    body::Body,
    extract::{Request, State},
    http::{StatusCode, header::AUTHORIZATION},
    middleware::{self, Next},
    response::Response,
};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;
use vek_core::SharedCore;

pub use error::{ApiError, ErrorBody};

/// Состояние API-приложения (клонируется на каждый запрос).
#[derive(Clone)]
pub struct ApiState {
    pub core: SharedCore,
    pub token: Arc<str>,
}

/// Метаданные OpenAPI-документа.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "VEK Torrent API",
        version = "1.0",
        description = "Внешний REST API VEK Torrent: поиск по rutracker и управление загрузками.",
    ),
    tags(
        (name = "search", description = "Поиск и просмотр раздач"),
        (name = "downloads", description = "Управление загрузками"),
        (name = "system", description = "Состояние приложения")
    )
)]
struct ApiDoc;

/// Собирает axum-приложение: защищённые маршруты + Swagger UI.
pub fn build_app(state: ApiState) -> axum::Router {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(routes::health))
        .routes(routes!(routes::status))
        .routes(routes!(routes::search))
        .routes(routes!(routes::topic))
        .routes(routes!(routes::categories))
        .routes(routes!(routes::downloads))
        .routes(routes!(routes::transfer))
        .routes(routes!(routes::topic_files))
        .routes(routes!(routes::add_topic))
        .routes(routes!(routes::add_url))
        .routes(routes!(routes::pause))
        .routes(routes!(routes::resume))
        .routes(routes!(routes::remove))
        .with_state(state.clone())
        .split_for_parts();

    let protected = router.layer(middleware::from_fn_with_state(state, require_auth));

    protected
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", api))
        // Разрешаем кросс-доменные запросы: API защищён токеном и слушает localhost.
        .layer(CorsLayer::permissive())
}

/// Middleware Bearer-авторизации. Пропускает health-check без токена.
async fn require_auth(
    State(state): State<ApiState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    if request.uri().path() == "/api/v1/health" {
        return Ok(next.run(request).await);
    }

    let presented = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "))
        .map(str::trim);

    match presented {
        Some(token) if token_matches(token, &state.token) => Ok(next.run(request).await),
        _ => Err(ApiError::unauthorized()),
    }
}

/// Сравнение токенов за постоянное время (защита от timing-атак).
fn token_matches(presented: &str, expected: &str) -> bool {
    let a = presented.as_bytes();
    let b = expected.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Запущенный HTTP-сервер API с адресом и управлением остановкой.
pub struct ApiServer {
    pub addr: SocketAddr,
    shutdown: tokio::sync::oneshot::Sender<()>,
    handle: tokio::task::JoinHandle<()>,
}

impl ApiServer {
    /// Останавливает сервер и дожидается завершения задачи.
    pub async fn stop(self) {
        let _ = self.shutdown.send(());
        let _ = self.handle.await;
    }
}

/// Поднимает API-сервер на `host:port` в фоновой задаче.
pub async fn serve(state: ApiState, host: &str, port: u16) -> std::io::Result<ApiServer> {
    let app = build_app(state);
    let listener = TcpListener::bind((host, port)).await?;
    let addr = listener.local_addr()?;

    let (shutdown, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let handle = tokio::spawn(async move {
        let server = axum::serve(listener, app).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });
        if let Err(e) = server.await {
            tracing::error!("API-сервер завершился с ошибкой: {e}");
        }
    });

    Ok(ApiServer {
        addr,
        shutdown,
        handle,
    })
}

/// Формирует 500-ответ (используется на верхнем уровне при необходимости).
pub fn internal_error(message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_matches_is_exact() {
        assert!(token_matches("abc123", "abc123"));
        assert!(!token_matches("abc123", "abc124"));
        assert!(!token_matches("abc", "abc123"));
        assert!(!token_matches("", "x"));
    }
}

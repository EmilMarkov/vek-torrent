//! HTTP-обработчики внешнего API. Тонкие адаптеры над [`vek_core::AppCore`].

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use vek_core::models::{AddOptions, AppStatus, DownloadItem, TransferSummary};
use vek_core::qbit_models::Category;
use vek_core::rutracker_models::{ForumGroup, SearchPage, SearchRequest, TopicPage};

use crate::{ApiState, error::ApiError};

/// Ответ health-check.
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthBody {
    pub status: String,
    pub name: String,
    pub version: String,
}

/// Тело запроса на добавление по ссылке.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AddUrlBody {
    /// magnet- или http(s)-ссылка на `.torrent`.
    pub url: String,
    #[serde(default)]
    pub options: AddOptions,
}

/// Тело запроса со списком хэшей.
#[derive(Debug, Deserialize, ToSchema)]
pub struct HashesBody {
    pub hashes: Vec<String>,
}

/// Тело запроса на удаление.
#[derive(Debug, Deserialize, ToSchema)]
pub struct DeleteBody {
    pub hashes: Vec<String>,
    #[serde(default)]
    pub delete_files: bool,
}

/// Проверка доступности сервиса (без авторизации).
#[utoipa::path(
    get, path = "/api/v1/health", tag = "system",
    responses((status = 200, description = "Сервис доступен", body = HealthBody))
)]
pub async fn health() -> Json<HealthBody> {
    Json(HealthBody {
        status: "ok".to_owned(),
        name: "VEK Torrent".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    })
}

/// Сводный статус подсистем приложения.
#[utoipa::path(
    get, path = "/api/v1/status", tag = "system",
    responses((status = 200, body = AppStatus))
)]
pub async fn status(State(state): State<ApiState>) -> Json<AppStatus> {
    Json(state.core.status().await)
}

/// Поиск по трекеру.
#[utoipa::path(
    post, path = "/api/v1/search", tag = "search",
    request_body = SearchRequest,
    responses(
        (status = 200, body = SearchPage),
        (status = 401, description = "Нет активной сессии", body = crate::ErrorBody),
        (status = 409, description = "Требуется капча", body = crate::ErrorBody)
    )
)]
pub async fn search(
    State(state): State<ApiState>,
    Json(request): Json<SearchRequest>,
) -> Result<Json<SearchPage>, ApiError> {
    Ok(Json(state.core.search(request).await?))
}

/// Страница раздачи.
#[utoipa::path(
    get, path = "/api/v1/topics/{id}", tag = "search",
    params(("id" = u64, Path, description = "Идентификатор раздачи")),
    responses((status = 200, body = TopicPage), (status = 404, body = crate::ErrorBody))
)]
pub async fn topic(
    State(state): State<ApiState>,
    Path(id): Path<u64>,
) -> Result<Json<TopicPage>, ApiError> {
    Ok(Json(state.core.topic(id).await?))
}

/// Дерево категорий (форумов) трекера.
#[utoipa::path(
    get, path = "/api/v1/categories", tag = "search",
    responses((status = 200, body = Vec<ForumGroup>))
)]
pub async fn categories(
    State(state): State<ApiState>,
) -> Result<Json<Vec<ForumGroup>>, ApiError> {
    Ok(Json(state.core.categories().await?))
}

/// Список загрузок.
#[utoipa::path(
    get, path = "/api/v1/downloads", tag = "downloads",
    responses((status = 200, body = Vec<DownloadItem>), (status = 503, body = crate::ErrorBody))
)]
pub async fn downloads(
    State(state): State<ApiState>,
) -> Result<Json<Vec<DownloadItem>>, ApiError> {
    Ok(Json(state.core.downloads().await?))
}

/// Глобальная статистика передачи.
#[utoipa::path(
    get, path = "/api/v1/transfer", tag = "downloads",
    responses((status = 200, body = TransferSummary), (status = 503, body = crate::ErrorBody))
)]
pub async fn transfer(
    State(state): State<ApiState>,
) -> Result<Json<TransferSummary>, ApiError> {
    Ok(Json(state.core.transfer().await?))
}

/// Категории qBittorrent.
#[utoipa::path(
    get, path = "/api/v1/qbit/categories", tag = "downloads",
    responses((status = 200, body = Vec<Category>), (status = 503, body = crate::ErrorBody))
)]
pub async fn qbit_categories(
    State(state): State<ApiState>,
) -> Result<Json<Vec<Category>>, ApiError> {
    Ok(Json(state.core.qbit_categories().await?))
}

/// Добавить раздачу в загрузки по идентификатору темы.
#[utoipa::path(
    post, path = "/api/v1/downloads/topic/{id}", tag = "downloads",
    params(("id" = u64, Path, description = "Идентификатор раздачи")),
    request_body = AddOptions,
    responses((status = 204, description = "Добавлено"), (status = 503, body = crate::ErrorBody))
)]
pub async fn add_topic(
    State(state): State<ApiState>,
    Path(id): Path<u64>,
    Json(options): Json<AddOptions>,
) -> Result<StatusCode, ApiError> {
    state.core.add_from_topic(id, options).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Добавить торрент по magnet/http-ссылке.
#[utoipa::path(
    post, path = "/api/v1/downloads/url", tag = "downloads",
    request_body = AddUrlBody,
    responses((status = 204, description = "Добавлено"), (status = 503, body = crate::ErrorBody))
)]
pub async fn add_url(
    State(state): State<ApiState>,
    Json(body): Json<AddUrlBody>,
) -> Result<StatusCode, ApiError> {
    state.core.add_url(body.url, body.options).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Поставить загрузки на паузу.
#[utoipa::path(
    post, path = "/api/v1/downloads/pause", tag = "downloads",
    request_body = HashesBody,
    responses((status = 204, description = "Готово"), (status = 503, body = crate::ErrorBody))
)]
pub async fn pause(
    State(state): State<ApiState>,
    Json(body): Json<HashesBody>,
) -> Result<StatusCode, ApiError> {
    state.core.pause(body.hashes).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Возобновить загрузки.
#[utoipa::path(
    post, path = "/api/v1/downloads/resume", tag = "downloads",
    request_body = HashesBody,
    responses((status = 204, description = "Готово"), (status = 503, body = crate::ErrorBody))
)]
pub async fn resume(
    State(state): State<ApiState>,
    Json(body): Json<HashesBody>,
) -> Result<StatusCode, ApiError> {
    state.core.resume(body.hashes).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Удалить загрузки (опционально с файлами).
#[utoipa::path(
    delete, path = "/api/v1/downloads", tag = "downloads",
    request_body = DeleteBody,
    responses((status = 204, description = "Удалено"), (status = 503, body = crate::ErrorBody))
)]
pub async fn remove(
    State(state): State<ApiState>,
    Json(body): Json<DeleteBody>,
) -> Result<StatusCode, ApiError> {
    state.core.remove(body.hashes, body.delete_files).await?;
    Ok(StatusCode::NO_CONTENT)
}

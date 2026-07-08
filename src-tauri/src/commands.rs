//! Команды Tauri: тонкие обёртки над [`vek_core::AppCore`].

use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;
use tauri::State;

use vek_core::AppConfig;
use vek_core::models::{
    AddOptions, AppStatus, DownloadItem, FavoriteItem, HistoryItem, TorrentFilesPreview,
    TransferSummary,
};
use vek_core::rutracker_models::{
    CaptchaAnswer, CaptchaChallenge, ForumGroup, SearchPage, SearchRequest, SessionInfo, TopicPage,
};

use crate::{
    error::{CommandError, CommandResult},
    state::AppState,
};

/// Результат попытки входа: успех либо требование капчи.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LoginOutcome {
    /// Вход выполнен.
    Ok,
    /// Требуется ввод капчи.
    Captcha { challenge: CaptchaChallenge },
}

/// Изображение (капча/постер) в виде data-URL.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageData {
    pub data_url: String,
}

// ── Конфигурация ────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> AppConfig {
    // Пароль не покидает ядро; токен API остаётся (нужен для интеграций).
    state.core.config_for_ui()
}

#[tauri::command]
pub async fn set_config(state: State<'_, AppState>, config: AppConfig) -> CommandResult<()> {
    state.core.update_config(config)?;
    Ok(())
}

// ── Сессия rutracker ────────────────────────────────────────────────────

#[tauri::command]
pub async fn session_status(state: State<'_, AppState>) -> CommandResult<SessionInfo> {
    Ok(state.core.session_status().await?)
}

#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    captcha: Option<CaptchaAnswer>,
) -> CommandResult<LoginOutcome> {
    match state.core.login(captcha).await {
        Ok(()) => Ok(LoginOutcome::Ok),
        Err(vek_core::Error::CaptchaRequired(challenge)) => {
            Ok(LoginOutcome::Captcha { challenge: *challenge })
        }
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> CommandResult<()> {
    state.core.logout()?;
    Ok(())
}

#[tauri::command]
pub async fn fetch_image(state: State<'_, AppState>, url: String) -> CommandResult<ImageData> {
    let (bytes, content_type) = state.core.fetch_image(&url).await?;
    let mime = content_type.unwrap_or_else(|| "image/jpeg".to_owned());
    let data_url = format!("data:{};base64,{}", mime, STANDARD.encode(&bytes));
    Ok(ImageData { data_url })
}

// ── Поиск и раздачи ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn search(
    state: State<'_, AppState>,
    request: SearchRequest,
) -> CommandResult<SearchPage> {
    Ok(state.core.search(request).await?)
}

#[tauri::command]
pub async fn topic(state: State<'_, AppState>, id: u64) -> CommandResult<TopicPage> {
    Ok(state.core.topic(id).await?)
}

#[tauri::command]
pub async fn categories(state: State<'_, AppState>) -> CommandResult<Vec<ForumGroup>> {
    Ok(state.core.categories().await?)
}

// ── Загрузки ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn downloads(state: State<'_, AppState>) -> CommandResult<Vec<DownloadItem>> {
    Ok(state.core.downloads().await?)
}

#[tauri::command]
pub async fn transfer(state: State<'_, AppState>) -> CommandResult<TransferSummary> {
    Ok(state.core.transfer().await?)
}

/// Список файлов раздачи (для выбора перед скачиванием).
#[tauri::command]
pub async fn topic_files(
    state: State<'_, AppState>,
    topic_id: u64,
) -> CommandResult<TorrentFilesPreview> {
    Ok(state.core.preview_topic(topic_id).await?)
}

#[tauri::command]
pub async fn add_from_topic(
    state: State<'_, AppState>,
    topic_id: u64,
    options: AddOptions,
) -> CommandResult<String> {
    Ok(state.core.add_from_topic(topic_id, options).await?)
}

#[tauri::command]
pub async fn add_url(
    state: State<'_, AppState>,
    url: String,
    options: AddOptions,
) -> CommandResult<String> {
    Ok(state.core.add_url(url, options).await?)
}

#[tauri::command]
pub async fn pause(state: State<'_, AppState>, hashes: Vec<String>) -> CommandResult<()> {
    state.core.pause(hashes).await?;
    Ok(())
}

#[tauri::command]
pub async fn resume(state: State<'_, AppState>, hashes: Vec<String>) -> CommandResult<()> {
    state.core.resume(hashes).await?;
    Ok(())
}

#[tauri::command]
pub async fn remove(
    state: State<'_, AppState>,
    hashes: Vec<String>,
    delete_files: bool,
) -> CommandResult<()> {
    state.core.remove(hashes, delete_files).await?;
    Ok(())
}

// ── Избранное и история ──────────────────────────────────────────────────

#[tauri::command]
pub fn favorites(state: State<'_, AppState>) -> Vec<FavoriteItem> {
    state.core.favorites()
}

#[tauri::command]
pub fn is_favorite(state: State<'_, AppState>, topic_id: u64) -> bool {
    state.core.is_favorite(topic_id)
}

#[tauri::command]
pub async fn add_favorite(state: State<'_, AppState>, topic_id: u64) -> CommandResult<()> {
    state.core.add_favorite(topic_id).await?;
    Ok(())
}

#[tauri::command]
pub fn remove_favorite(state: State<'_, AppState>, topic_id: u64) {
    state.core.remove_favorite(topic_id);
}

#[tauri::command]
pub fn clear_favorite_update(state: State<'_, AppState>, topic_id: u64) {
    state.core.clear_favorite_update(topic_id);
}

#[tauri::command]
pub async fn check_favorites(state: State<'_, AppState>) -> CommandResult<Vec<FavoriteItem>> {
    Ok(state.core.check_favorites().await?)
}

#[tauri::command]
pub fn history(state: State<'_, AppState>) -> Vec<HistoryItem> {
    state.core.history()
}

#[tauri::command]
pub fn remove_history(state: State<'_, AppState>, topic_id: u64) {
    state.core.remove_history(topic_id);
}

#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) {
    state.core.clear_history();
}

// ── Система ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn status(state: State<'_, AppState>) -> CommandResult<AppStatus> {
    Ok(state.core.status().await)
}

#[tauri::command]
pub async fn start_engine(state: State<'_, AppState>) -> CommandResult<()> {
    state.core.ensure_engine().await?;
    Ok(())
}

#[tauri::command]
pub async fn stop_engine(state: State<'_, AppState>) -> CommandResult<()> {
    state.core.stop_engine().await;
    Ok(())
}

/// Перезапускает внешний API согласно текущей конфигурации.
#[tauri::command]
pub async fn restart_api(state: State<'_, AppState>) -> CommandResult<()> {
    state
        .restart_api()
        .await
        .map_err(|e| CommandError::new("api_error", e))
}

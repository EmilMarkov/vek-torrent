//! Команды Tauri: тонкие обёртки над [`vek_core::AppCore`].

use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;
use tauri::State;

use vek_core::AppConfig;
use vek_core::models::{AddOptions, AppStatus, DownloadItem, TransferSummary};
use vek_core::qbit_models::Category;
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
    state.core.config()
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

#[tauri::command]
pub async fn qbit_categories(state: State<'_, AppState>) -> CommandResult<Vec<Category>> {
    Ok(state.core.qbit_categories().await?)
}

#[tauri::command]
pub async fn add_from_topic(
    state: State<'_, AppState>,
    topic_id: u64,
    options: AddOptions,
) -> CommandResult<()> {
    state.core.add_from_topic(topic_id, options).await?;
    Ok(())
}

#[tauri::command]
pub async fn add_url(
    state: State<'_, AppState>,
    url: String,
    options: AddOptions,
) -> CommandResult<()> {
    state.core.add_url(url, options).await?;
    Ok(())
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

// ── Система ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn status(state: State<'_, AppState>) -> AppStatus {
    state.core.status().await
}

#[tauri::command]
pub async fn start_qbit(state: State<'_, AppState>) -> CommandResult<()> {
    state.core.ensure_qbit().await?;
    Ok(())
}

#[tauri::command]
pub async fn stop_qbit(state: State<'_, AppState>) -> CommandResult<()> {
    state.core.stop_qbit().await;
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

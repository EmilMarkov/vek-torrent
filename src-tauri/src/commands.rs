//! Команды Tauri: тонкие обёртки над [`vek_core::AppCore`].

use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;
use tauri::State;

use vek_core::AppConfig;
use vek_core::models::{
    AddOptions, AppStatus, CategoryItem, ChangeEventItem, DownloadItem, FavoriteItem,
    FileVersionInfo, FolderItem, HistoryItem, MirrorStatus, PatchInfo, TorrentFilesPreview,
    TransferSummary, VersionMatch,
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

/// Генерирует новый токен внешнего API (старый перестаёт действовать
/// после перезапуска API).
#[tauri::command]
pub fn regenerate_api_token(state: State<'_, AppState>) -> CommandResult<String> {
    Ok(state.core.regenerate_api_token()?)
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
        Err(vek_core::Error::CaptchaRequired(challenge)) => Ok(LoginOutcome::Captcha {
            challenge: *challenge,
        }),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> CommandResult<()> {
    state.core.logout()?;
    Ok(())
}

/// Проверяет доступность зеркал rutracker через текущий прокси.
#[tauri::command]
pub async fn check_mirrors(state: State<'_, AppState>) -> CommandResult<Vec<MirrorStatus>> {
    Ok(state.core.check_mirrors().await)
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

/// Сохраняет `.torrent`-файл раздачи по пути, выбранному пользователем.
#[tauri::command]
pub async fn save_torrent(
    state: State<'_, AppState>,
    topic_id: u64,
    path: String,
) -> CommandResult<Option<String>> {
    Ok(state.core.save_torrent_file(topic_id, path).await?)
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

/// История изменений отслеживаемой раздачи.
#[tauri::command]
pub fn favorite_history(state: State<'_, AppState>, topic_id: u64) -> Vec<ChangeEventItem> {
    state.core.favorite_history(topic_id)
}

/// Сохранённые версии списков файлов раздачи.
#[tauri::command]
pub fn tracked_versions(state: State<'_, AppState>, topic_id: u64) -> Vec<FileVersionInfo> {
    state.core.tracked_versions(topic_id)
}

/// Изменения файлов между версией пользователя и последней сохранённой.
/// Версия адресуется временем фиксации (`base_at`) — стабильно к вытеснению.
#[tauri::command]
pub fn compute_patch(
    state: State<'_, AppState>,
    topic_id: u64,
    base_at: i64,
) -> CommandResult<PatchInfo> {
    Ok(state.core.compute_patch(topic_id, base_at)?)
}

/// Определяет скачанную версию по выбранной локальной папке.
#[tauri::command]
pub async fn detect_version(
    state: State<'_, AppState>,
    topic_id: u64,
    dir: String,
) -> CommandResult<Vec<VersionMatch>> {
    // Сканирование папки — блокирующее I/O, уводим с async-потока.
    let core = state.core.clone();
    Ok(
        tauri::async_runtime::spawn_blocking(move || core.detect_version(topic_id, dir))
            .await
            .map_err(|e| CommandError::new("error", e.to_string()))??,
    )
}

/// Скачивает патч: только изменённые файлы актуальной раздачи.
#[tauri::command]
pub async fn download_patch(
    state: State<'_, AppState>,
    topic_id: u64,
    base_at: i64,
    options: AddOptions,
) -> CommandResult<String> {
    Ok(state
        .core
        .download_patch(topic_id, base_at, options)
        .await?)
}

// ── Папки и категории ────────────────────────────────────────────────────

#[tauri::command]
pub fn user_categories(state: State<'_, AppState>) -> Vec<CategoryItem> {
    state.core.user_categories()
}

#[tauri::command]
pub fn add_user_category(
    state: State<'_, AppState>,
    name: String,
    color: String,
    forum_ids: Vec<i64>,
) -> CommandResult<CategoryItem> {
    Ok(state.core.add_user_category(name, color, forum_ids)?)
}

#[tauri::command]
pub fn update_user_category(
    state: State<'_, AppState>,
    id: String,
    name: String,
    color: String,
    forum_ids: Vec<i64>,
) -> CommandResult<()> {
    state
        .core
        .update_user_category(id, name, color, forum_ids)?;
    Ok(())
}

#[tauri::command]
pub fn remove_user_category(state: State<'_, AppState>, id: String) {
    state.core.remove_user_category(id);
}

#[tauri::command]
pub fn folders(state: State<'_, AppState>) -> Vec<FolderItem> {
    state.core.folders()
}

#[tauri::command]
pub fn add_folder(
    state: State<'_, AppState>,
    name: String,
    category_id: Option<String>,
) -> CommandResult<()> {
    state.core.add_folder(name, category_id)?;
    Ok(())
}

#[tauri::command]
pub fn update_folder(
    state: State<'_, AppState>,
    id: String,
    name: String,
    category_id: Option<String>,
) -> CommandResult<()> {
    state.core.update_folder(id, name, category_id)?;
    Ok(())
}

#[tauri::command]
pub fn remove_folder(state: State<'_, AppState>, id: String) {
    state.core.remove_folder(id);
}

#[tauri::command]
pub fn add_topic_to_folder(
    state: State<'_, AppState>,
    folder_id: String,
    topic_id: u64,
    title: String,
) -> CommandResult<()> {
    state.core.add_topic_to_folder(folder_id, topic_id, title)?;
    Ok(())
}

#[tauri::command]
pub fn remove_topic_from_folder(state: State<'_, AppState>, folder_id: String, topic_id: u64) {
    state.core.remove_topic_from_folder(folder_id, topic_id);
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

/// Забирает отложенную тему из внутренней ссылки (при холодном старте).
#[tauri::command]
pub fn take_pending_deeplink(state: State<'_, AppState>) -> Option<u64> {
    state.take_pending_deeplink()
}

/// Перезапускает внешний API согласно текущей конфигурации.
#[tauri::command]
pub async fn restart_api(state: State<'_, AppState>) -> CommandResult<()> {
    state
        .restart_api()
        .await
        .map_err(|e| CommandError::new("api_error", e))
}

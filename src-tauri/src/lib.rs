//! Точка сборки приложения VEK Torrent: состояние, команды, события, плагины.

mod commands;
mod error;
mod state;

use std::time::Duration;

use serde::Serialize;
use tauri::{Emitter, Manager, RunEvent};
use vek_core::{
    AppCore,
    models::{DownloadItem, TransferSummary},
};

use state::AppState;

/// Полезная нагрузка события обновления загрузок.
#[derive(Debug, Clone, Serialize)]
struct DownloadsUpdate {
    items: Vec<DownloadItem>,
    transfer: TransferSummary,
}

/// Интервал фонового опроса загрузок.
const POLL_INTERVAL: Duration = Duration::from_millis(1500);

/// Интервал автоматической проверки обновлений избранного.
const FAVORITES_CHECK_INTERVAL: Duration = Duration::from_secs(3 * 60 * 60);

/// Запуск приложения Tauri.
pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("не удалось определить каталог данных: {e}"))?;

            let core = AppCore::new(app_dir)?;
            app.manage(AppState::new(core.clone()));

            spawn_background_tasks(app.handle().clone(), core);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::set_config,
            commands::session_status,
            commands::login,
            commands::logout,
            commands::fetch_image,
            commands::search,
            commands::topic,
            commands::categories,
            commands::downloads,
            commands::transfer,
            commands::topic_files,
            commands::add_from_topic,
            commands::add_url,
            commands::pause,
            commands::resume,
            commands::remove,
            commands::favorites,
            commands::is_favorite,
            commands::add_favorite,
            commands::remove_favorite,
            commands::clear_favorite_update,
            commands::check_favorites,
            commands::history,
            commands::remove_history,
            commands::clear_history,
            commands::status,
            commands::start_engine,
            commands::stop_engine,
            commands::restart_api,
        ])
        .build(tauri::generate_context!())
        .expect("не удалось собрать приложение VEK Torrent")
        .run(|app_handle, event| {
            if let RunEvent::Exit = event {
                // Корректно останавливаем движок и API при выходе.
                let state = app_handle.state::<AppState>();
                tauri::async_runtime::block_on(async {
                    state.stop_api().await;
                    state.core.stop_engine().await;
                });
            }
        });
}

/// Запускает фоновые задачи: автозапуск движка/API и опрос загрузок.
fn spawn_background_tasks(app: tauri::AppHandle, core: vek_core::SharedCore) {
    // Автозапуск подсистем согласно конфигурации.
    {
        let app = app.clone();
        let core = core.clone();
        tauri::async_runtime::spawn(async move {
            let config = core.config();
            if config.engine.autostart
                && let Err(e) = core.ensure_engine().await
            {
                tracing::warn!("не удалось автозапустить торрент-движок: {e}");
            }
            if config.api.enabled {
                let state = app.state::<AppState>();
                if let Err(e) = state.restart_api().await {
                    tracing::warn!("не удалось запустить API при старте: {e}");
                }
            }
        });
    }

    // Периодическая проверка обновлений избранных раздач.
    {
        let app = app.clone();
        let core = core.clone();
        tauri::async_runtime::spawn(async move {
            // Первая проверка — через минуту после старта, далее по интервалу.
            tokio::time::sleep(Duration::from_secs(60)).await;
            let mut interval = tokio::time::interval(FAVORITES_CHECK_INTERVAL);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                if let Ok(favorites) = core.check_favorites().await {
                    let _ = app.emit("favorites:updated", favorites);
                }
            }
        });
    }

    // Фоновый опрос загрузок с рассылкой событий во фронтенд.
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if let Some((items, transfer)) = core.snapshot().await {
                let _ = app.emit("downloads:update", DownloadsUpdate { items, transfer });
            }
        }
    });
}

/// Инициализирует подсистему логирования (однократно, без паники при повторе).
fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .try_init();
}

//! Точка сборки приложения VEK Torrent: состояние, команды, события, плагины.

mod commands;
mod error;
mod state;

use std::{collections::HashSet, time::Duration};

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, RunEvent, WindowEvent,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_notification::NotificationExt;
use vek_core::{
    AppCore,
    models::{DownloadItem, DownloadState, TransferSummary},
};

use state::AppState;

/// Разбирает внутреннюю ссылку `vektorrent://topic/<id>` в идентификатор темы.
fn parse_topic_url(url: &str) -> Option<u64> {
    let rest = url.strip_prefix("vektorrent://topic/")?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Показывает и фокусирует главное окно.
fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Отправляет нативное уведомление ОС (ошибки игнорируются).
fn notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

/// Создаёт значок в системном трее с меню и обработкой кликов.
fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Показать окно", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

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

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default();

    // Единственный экземпляр (Windows/Linux): второй запуск фокусирует окно.
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }));
    }

    builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("не удалось определить каталог данных: {e}"))?;

            let core = AppCore::new(app_dir)?;
            app.manage(AppState::new(core.clone()));

            // Обработка внутренних ссылок vektorrent://topic/<id>.
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            let _ = app.deep_link().register_all();

            // Холодный старт: ссылка, которой открыли приложение.
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                for url in urls {
                    if let Some(id) = parse_topic_url(url.as_str()) {
                        app.state::<AppState>().set_pending_deeplink(id);
                    }
                }
            }

            // Приложение уже запущено: сразу переходим на тему.
            let deep_link_handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    if let Some(id) = parse_topic_url(url.as_str()) {
                        let _ = deep_link_handle.emit("open-topic", id);
                    }
                }
            });

            // Значок в системном трее с меню.
            if let Err(e) = setup_tray(app.handle()) {
                tracing::warn!("не удалось создать значок в трее: {e}");
            }

            spawn_background_tasks(app.handle().clone(), core);
            Ok(())
        })
        // Закрытие окна сворачивает приложение в трей (выход — через меню трея).
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::set_config,
            commands::check_mirrors,
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
            commands::save_torrent,
            commands::pause,
            commands::resume,
            commands::remove,
            commands::favorites,
            commands::is_favorite,
            commands::add_favorite,
            commands::remove_favorite,
            commands::clear_favorite_update,
            commands::check_favorites,
            commands::favorite_history,
            commands::tracked_versions,
            commands::compute_patch,
            commands::detect_version,
            commands::download_patch,
            commands::user_categories,
            commands::add_user_category,
            commands::update_user_category,
            commands::remove_user_category,
            commands::folders,
            commands::add_folder,
            commands::update_folder,
            commands::remove_folder,
            commands::add_topic_to_folder,
            commands::remove_topic_from_folder,
            commands::history,
            commands::remove_history,
            commands::clear_history,
            commands::status,
            commands::start_engine,
            commands::stop_engine,
            commands::take_pending_deeplink,
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

    // Периодическая проверка обновлений избранных раздач + уведомления.
    {
        let app = app.clone();
        let core = core.clone();
        tauri::async_runtime::spawn(async move {
            // Первая проверка — через минуту после старта, далее по интервалу.
            tokio::time::sleep(Duration::from_secs(60)).await;
            let mut interval = tokio::time::interval(FAVORITES_CHECK_INTERVAL);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            // Первый прогон только «прогревает» набор, чтобы не спамить об уже
            // накопленных обновлениях при запуске.
            let mut notified: HashSet<u64> = HashSet::new();
            let mut primed = false;
            loop {
                interval.tick().await;
                if let Ok(favorites) = core.check_favorites().await {
                    for fav in &favorites {
                        if fav.has_update {
                            if notified.insert(fav.topic_id) && primed {
                                notify(&app, "Обновление раздачи", &fav.title);
                            }
                        } else {
                            notified.remove(&fav.topic_id);
                        }
                    }
                    primed = true;
                    let _ = app.emit("favorites:updated", favorites);
                }
            }
        });
    }

    // Фоновый опрос загрузок: события во фронтенд + уведомления о статусах.
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut done: HashSet<String> = HashSet::new();
        let mut errored: HashSet<String> = HashSet::new();
        let mut primed = false;
        loop {
            interval.tick().await;
            if let Some((items, transfer)) = core.snapshot().await {
                for item in &items {
                    if item.finished && done.insert(item.hash.clone()) && primed {
                        notify(&app, "Загрузка завершена", &item.name);
                    }
                    if matches!(item.state, DownloadState::Error)
                        && errored.insert(item.hash.clone())
                        && primed
                    {
                        notify(&app, "Ошибка загрузки", &item.name);
                    }
                }
                primed = true;
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

//! Точка сборки приложения VEK Torrent: плагины, состояние, команды.
//!
//! Команды и состояние подключаются на этапе 4.

/// Запуск приложения Tauri.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("не удалось запустить приложение VEK Torrent");
}

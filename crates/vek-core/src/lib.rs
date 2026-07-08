//! Доменный слой VEK Torrent.
//!
//! Объединяет клиент rutracker и sidecar qBittorrent в единое ядро
//! [`AppCore`], предоставляя операции поиска, просмотра раздач и управления
//! загрузками для слоя приложения (Tauri) и внешнего REST API.

pub mod app;
pub mod config;
pub mod error;
pub mod models;
pub mod sidecar;

pub use app::{AppCore, SharedCore};
pub use config::AppConfig;
pub use error::{Error, Result};

// Реэкспорт моделей rutracker и qBittorrent для удобства верхних слоёв.
pub use qbit::models as qbit_models;
pub use rutracker::models as rutracker_models;

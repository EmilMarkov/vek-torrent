//! Доменный слой VEK Torrent.
//!
//! Объединяет клиент rutracker и встроенный торрент-движок в единое ядро
//! [`AppCore`], предоставляя операции поиска, просмотра раздач и управления
//! загрузками для слоя приложения (Tauri) и внешнего REST API.
//!
//! Точка входа — [`AppCore`]; конфигурация — [`AppConfig`].

pub mod app;
pub mod config;
pub mod error;
pub mod library;
pub mod models;

pub use app::{AppCore, SharedCore};
pub use config::AppConfig;
pub use error::{Error, Result};

// Реэкспорт моделей rutracker для удобства верхних слоёв.
pub use rutracker::models as rutracker_models;

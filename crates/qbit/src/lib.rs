//! Типизированный клиент qBittorrent Web API v2.
//!
//! Ориентирован на qBittorrent 5.x, но совместим с 4.x: методы «остановить»
//! и «запустить» автоматически выбирают между `stop/start` (v5) и
//! `pause/resume` (v4) с откатом по коду ответа 404/405.
//!
//! Основной способ получения состояния загрузок — периодический опрос
//! [`Client::torrents`] (надёжнее, чем диффы `sync/maindata`) в паре с
//! [`Client::transfer_info`] для глобальной статистики.

pub mod client;
pub mod error;
pub mod models;

pub use client::{Client, ClientConfig};
pub use error::Error;

/// Результат операций клиента qBittorrent.
pub type Result<T> = std::result::Result<T, Error>;

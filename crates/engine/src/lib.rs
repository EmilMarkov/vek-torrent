//! Встроенный торрент-движок VEK Torrent на базе [`librqbit`].
//!
//! Инкапсулирует сессию librqbit и предоставляет простой асинхронный API:
//! запуск, добавление (magnet/`.torrent`), предпросмотр файлов, список
//! состояний, пауза/возобновление/удаление. Заменяет прежнюю интеграцию с
//! внешним qBittorrent (sidecar).

pub mod engine;
pub mod error;
pub mod models;

pub use engine::{Engine, EngineConfig};
pub use error::{Error, Result};
pub use models::{AddParams, EngineFile, EngineTorrent, Source, TorrentPreview, TorrentState};

//! Модели данных qBittorrent Web API v2.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Информация о торренте (подмножество полей `/torrents/info`).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TorrentInfo {
    pub hash: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub size: i64,
    /// Прогресс 0.0..=1.0.
    #[serde(default)]
    pub progress: f64,
    /// Скорость скачивания, байт/с.
    #[serde(default)]
    pub dlspeed: i64,
    /// Скорость раздачи, байт/с.
    #[serde(default)]
    pub upspeed: i64,
    /// Оставшееся время, секунды (8640000 — «∞»).
    #[serde(default)]
    pub eta: i64,
    /// Строковое состояние qBittorrent (см. [`TorrentState`]).
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: String,
    #[serde(default)]
    pub save_path: String,
    #[serde(default)]
    pub num_seeds: i64,
    #[serde(default)]
    pub num_leechs: i64,
    #[serde(default)]
    pub ratio: f64,
    /// Байт скачано из нужного.
    #[serde(default)]
    pub completed: i64,
    #[serde(default)]
    pub amount_left: i64,
    /// Момент добавления, unix-время.
    #[serde(default)]
    pub added_on: i64,
    /// Момент завершения, unix-время (-1 если не завершён).
    #[serde(default)]
    pub completion_on: i64,
}

impl TorrentInfo {
    /// Нормализованная категория состояния.
    pub fn state_kind(&self) -> TorrentState {
        TorrentState::from_raw(&self.state)
    }
}

/// Нормализованная категория состояния торрента.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TorrentState {
    Downloading,
    Uploading,
    /// Стоит в очереди.
    Queued,
    /// На паузе/остановлен.
    Paused,
    /// Проверка данных.
    Checking,
    /// Загрузка метаданных (magnet).
    Metadata,
    /// Перемещение файлов.
    Moving,
    Error,
    Unknown,
}

impl TorrentState {
    /// Классифицирует «сырое» состояние qBittorrent (совместимо с v4 и v5).
    pub fn from_raw(raw: &str) -> Self {
        match raw {
            "downloading" | "forcedDL" | "stalledDL" => Self::Downloading,
            "uploading" | "forcedUP" | "stalledUP" => Self::Uploading,
            "queuedDL" | "queuedUP" => Self::Queued,
            // v4: paused*, v5: stopped*
            "pausedDL" | "pausedUP" | "stoppedDL" | "stoppedUP" => Self::Paused,
            "checkingDL" | "checkingUP" | "checkingResumeData" | "allocating" => Self::Checking,
            "metaDL" | "forcedMetaDL" => Self::Metadata,
            "moving" => Self::Moving,
            "error" | "missingFiles" => Self::Error,
            _ => Self::Unknown,
        }
    }

    /// Активна ли загрузка (для решения о поллинге/иконке).
    pub fn is_active(self) -> bool {
        matches!(
            self,
            Self::Downloading | Self::Uploading | Self::Checking | Self::Metadata | Self::Moving
        )
    }
}

/// Глобальная статистика передачи (`/transfer/info`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct TransferInfo {
    #[serde(default)]
    pub dl_info_speed: i64,
    #[serde(default)]
    pub up_info_speed: i64,
    #[serde(default)]
    pub dl_info_data: i64,
    #[serde(default)]
    pub up_info_data: i64,
    /// Состояние соединения: `connected` / `firewalled` / `disconnected`.
    #[serde(default)]
    pub connection_status: String,
}

/// Категория qBittorrent.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Category {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "savePath")]
    pub save_path: String,
}

/// Фильтр состояния для запроса списка торрентов.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TorrentFilter {
    #[default]
    All,
    Downloading,
    Completed,
    Seeding,
    Active,
    Inactive,
    Paused,
    Resumed,
    Errored,
}

impl TorrentFilter {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Downloading => "downloading",
            Self::Completed => "completed",
            Self::Seeding => "seeding",
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Paused => "paused",
            Self::Resumed => "resumed",
            Self::Errored => "errored",
        }
    }
}

/// Параметры запроса списка торрентов.
#[derive(Debug, Clone, Default)]
pub struct TorrentsQuery {
    pub filter: TorrentFilter,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub sort: Option<String>,
    pub reverse: bool,
    pub limit: Option<u32>,
    pub offset: Option<i32>,
    /// Ограничить конкретными хэшами.
    pub hashes: Vec<String>,
}

/// Источник добавляемого торрента.
#[derive(Debug, Clone)]
pub enum TorrentSource {
    /// magnet-ссылка или http(s)-ссылка на `.torrent`.
    Url(String),
    /// Содержимое `.torrent`-файла.
    File { filename: String, bytes: Vec<u8> },
}

/// Параметры добавления торрента.
#[derive(Debug, Clone, Default)]
pub struct AddTorrent {
    pub sources: Vec<TorrentSource>,
    /// Каталог сохранения (иначе — по умолчанию из настроек qBittorrent).
    pub save_path: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    /// Добавить остановленным.
    pub stopped: bool,
    /// Не запускать проверку хэшей (skip_checking).
    pub skip_checking: bool,
}

//! DTO доменного слоя, отдаваемые в UI и внешний API (camelCase).

use engine::{EngineFile, EngineTorrent, TorrentState as EngineState};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Нормализованное состояние загрузки (для UI/API).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DownloadState {
    Downloading,
    Seeding,
    Paused,
    Checking,
    Error,
    Unknown,
}

impl From<EngineState> for DownloadState {
    fn from(state: EngineState) -> Self {
        match state {
            EngineState::Downloading => Self::Downloading,
            EngineState::Seeding => Self::Seeding,
            EngineState::Paused => Self::Paused,
            EngineState::Checking => Self::Checking,
            EngineState::Error => Self::Error,
            EngineState::Unknown => Self::Unknown,
        }
    }
}

/// Элемент списка загрузок (UI-дружественная проекция состояния движка).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DownloadItem {
    pub hash: String,
    pub name: String,
    pub size: u64,
    /// Прогресс 0.0..=1.0.
    pub progress: f64,
    pub dlspeed: u64,
    pub upspeed: u64,
    /// Оставшееся время, секунды (None — неизвестно/бесконечно).
    pub eta: Option<u64>,
    pub state: DownloadState,
    pub save_path: String,
    pub num_peers: u32,
    pub downloaded: u64,
    pub uploaded: u64,
    pub finished: bool,
    pub error: Option<String>,
}

impl From<EngineTorrent> for DownloadItem {
    fn from(t: EngineTorrent) -> Self {
        Self {
            hash: t.hash,
            name: t.name,
            size: t.size,
            progress: t.progress,
            dlspeed: t.dl_speed,
            upspeed: t.up_speed,
            eta: t.eta_secs,
            state: t.state.into(),
            save_path: t.save_path,
            num_peers: t.peers,
            downloaded: t.downloaded,
            uploaded: t.uploaded,
            finished: t.finished,
            error: t.error,
        }
    }
}

/// Глобальная статистика передачи (UI-проекция).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TransferSummary {
    pub dl_speed: u64,
    pub up_speed: u64,
    /// Число активных загрузок.
    pub active: u32,
    /// Всего торрентов в списке.
    pub total: u32,
}

/// Файл внутри торрента (для выбора при добавлении).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TorrentFile {
    pub index: usize,
    pub path: String,
    pub size: u64,
}

impl From<EngineFile> for TorrentFile {
    fn from(f: EngineFile) -> Self {
        Self {
            index: f.index,
            path: f.path,
            size: f.size,
        }
    }
}

/// Предпросмотр раздачи со списком файлов (перед скачиванием).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TorrentFilesPreview {
    pub hash: String,
    pub name: String,
    pub total_size: u64,
    pub files: Vec<TorrentFile>,
}

/// Параметры добавления раздачи в загрузки.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddOptions {
    /// Каталог сохранения (иначе — из конфигурации).
    pub save_path: Option<String>,
    /// Добавить остановленным (иначе — из конфигурации).
    pub stopped: Option<bool>,
    /// Предпочесть magnet вместо скачивания `.torrent`-файла.
    #[serde(default)]
    pub prefer_magnet: bool,
    /// Индексы файлов для скачивания (None/пусто — все файлы).
    #[serde(default)]
    pub only_files: Option<Vec<usize>>,
}

/// Избранная раздача (проекция для UI/API).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteItem {
    pub topic_id: u64,
    pub title: String,
    pub added_at: i64,
    pub last_checked: i64,
    /// Обнаружено обновление раздачи с последнего просмотра.
    pub has_update: bool,
}

impl From<crate::library::FavoriteRecord> for FavoriteItem {
    fn from(r: crate::library::FavoriteRecord) -> Self {
        Self {
            topic_id: r.topic_id,
            title: r.title,
            added_at: r.added_at,
            last_checked: r.last_checked,
            has_update: r.has_update,
        }
    }
}

/// Запись истории скачиваний (проекция для UI/API).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub topic_id: u64,
    pub title: String,
    pub hash: String,
    pub added_at: i64,
}

impl From<crate::library::HistoryRecord> for HistoryItem {
    fn from(r: crate::library::HistoryRecord) -> Self {
        Self {
            topic_id: r.topic_id,
            title: r.title,
            hash: r.hash,
            added_at: r.added_at,
        }
    }
}

/// Статус подсистем приложения.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub engine_running: bool,
    pub api_running: bool,
    pub logged_in: bool,
    pub username: Option<String>,
    /// Число активных загрузок.
    pub active_downloads: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(state: EngineState, finished: bool, eta: Option<u64>) -> EngineTorrent {
        EngineTorrent {
            hash: "abc".into(),
            name: "torrent".into(),
            size: 100,
            progress: 0.5,
            downloaded: 50,
            uploaded: 10,
            dl_speed: 1000,
            up_speed: 100,
            eta_secs: eta,
            state,
            peers: 3,
            finished,
            save_path: "/d".into(),
            error: None,
        }
    }

    #[test]
    fn maps_engine_torrent_to_download_item() {
        let item = DownloadItem::from(sample(EngineState::Downloading, false, Some(120)));
        assert_eq!(item.hash, "abc");
        assert_eq!(item.state, DownloadState::Downloading);
        assert_eq!(item.eta, Some(120));
        assert_eq!(item.num_peers, 3);
        assert_eq!(item.dlspeed, 1000);
    }

    #[test]
    fn maps_seeding_state() {
        let item = DownloadItem::from(sample(EngineState::Seeding, true, None));
        assert_eq!(item.state, DownloadState::Seeding);
        assert!(item.finished);
        assert_eq!(item.eta, None);
    }
}

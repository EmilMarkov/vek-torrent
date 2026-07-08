//! DTO доменного слоя, отдаваемые в UI и внешний API (camelCase).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use qbit::models::{TorrentInfo, TorrentState, TransferInfo};

/// Элемент списка загрузок (UI-дружественная проекция [`TorrentInfo`]).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DownloadItem {
    pub hash: String,
    pub name: String,
    pub size: i64,
    /// Прогресс 0.0..=1.0.
    pub progress: f64,
    pub dlspeed: i64,
    pub upspeed: i64,
    /// Оставшееся время, секунды (None — неизвестно/бесконечно).
    pub eta: Option<i64>,
    pub state: TorrentState,
    pub state_raw: String,
    pub category: String,
    pub save_path: String,
    pub num_seeds: i64,
    pub num_leechs: i64,
    pub ratio: f64,
    pub added_on: i64,
    pub completion_on: i64,
}

/// «Бесконечное» ETA в qBittorrent.
const ETA_INFINITY: i64 = 8_640_000;

impl From<TorrentInfo> for DownloadItem {
    fn from(t: TorrentInfo) -> Self {
        let state = t.state_kind();
        let eta = if t.eta <= 0 || t.eta >= ETA_INFINITY {
            None
        } else {
            Some(t.eta)
        };
        Self {
            hash: t.hash,
            name: t.name,
            size: t.size,
            progress: t.progress,
            dlspeed: t.dlspeed,
            upspeed: t.upspeed,
            eta,
            state,
            state_raw: t.state,
            category: t.category,
            save_path: t.save_path,
            num_seeds: t.num_seeds,
            num_leechs: t.num_leechs,
            ratio: t.ratio,
            added_on: t.added_on,
            completion_on: t.completion_on,
        }
    }
}

/// Глобальная статистика передачи (UI-проекция).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TransferSummary {
    pub dl_speed: i64,
    pub up_speed: i64,
    pub dl_data: i64,
    pub up_data: i64,
    pub connection_status: String,
}

impl From<TransferInfo> for TransferSummary {
    fn from(t: TransferInfo) -> Self {
        Self {
            dl_speed: t.dl_info_speed,
            up_speed: t.up_info_speed,
            dl_data: t.dl_info_data,
            up_data: t.up_info_data,
            connection_status: t.connection_status,
        }
    }
}

/// Параметры добавления раздачи в загрузки.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddOptions {
    /// Каталог сохранения (иначе — из конфигурации/qBittorrent).
    pub save_path: Option<String>,
    pub category: Option<String>,
    /// Добавить остановленным (иначе — из конфигурации).
    pub stopped: Option<bool>,
    /// Предпочесть magnet вместо скачивания `.torrent`-файла.
    #[serde(default)]
    pub prefer_magnet: bool,
}

/// Статус подсистем приложения.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub qbit_running: bool,
    pub qbit_version: Option<String>,
    pub api_running: bool,
    pub logged_in: bool,
    pub username: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(state: &str, eta: i64) -> TorrentInfo {
        TorrentInfo {
            hash: "h".into(),
            name: "n".into(),
            size: 100,
            progress: 0.5,
            dlspeed: 10,
            upspeed: 1,
            eta,
            state: state.into(),
            category: String::new(),
            tags: String::new(),
            save_path: "/d".into(),
            num_seeds: 3,
            num_leechs: 1,
            ratio: 0.2,
            completed: 50,
            amount_left: 50,
            added_on: 100,
            completion_on: -1,
        }
    }

    #[test]
    fn maps_infinite_eta_to_none() {
        let item = DownloadItem::from(sample("stalledDL", ETA_INFINITY));
        assert_eq!(item.eta, None);
        assert_eq!(item.state, TorrentState::Downloading);
    }

    #[test]
    fn keeps_finite_eta() {
        let item = DownloadItem::from(sample("downloading", 120));
        assert_eq!(item.eta, Some(120));
        assert_eq!(item.state_raw, "downloading");
    }
}

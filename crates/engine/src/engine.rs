//! Обёртка над librqbit: запуск сессии, добавление, список, управление.

use std::{
    path::PathBuf,
    sync::{Arc, Once},
};

use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ManagedTorrent, Session, SessionOptions,
    SessionPersistenceConfig, TorrentStatsState, api::TorrentIdOrHash,
};

/// Дескриптор управляемого торрента (в librqbit — `Arc<ManagedTorrent>`).
type Handle = Arc<ManagedTorrent>;

use crate::{
    error::{Error, Result},
    models::{AddParams, EngineFile, EngineTorrent, Source, TorrentPreview, TorrentState},
};

/// Конфигурация движка.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Каталог сохранения загрузок по умолчанию.
    pub download_dir: PathBuf,
    /// Каталог для состояния сессии (персистентность торрентов).
    pub state_dir: PathBuf,
    /// Порт для входящих соединений; None — выбрать автоматически.
    pub listen_port: Option<u16>,
}

/// Встроенный торрент-движок.
#[derive(Clone)]
pub struct Engine {
    session: Arc<Session>,
    download_dir: PathBuf,
}

/// Однократно поднимает мягкий лимит открытых файлов (`RLIMIT_NOFILE`).
///
/// Файловое хранилище librqbit держит открытый дескриптор на *каждый* файл
/// раздачи. Крупные раздачи (игры с тысячами файлов) упираются в мягкий лимит:
/// у GUI-приложений на macOS он всего 256, и открытие очередного файла падает
/// с «error opening … in read/write mode» — на случайном файле, где счётчик
/// исчерпался. Поднимаем лимит до жёсткого (с учётом `kern.maxfilesperproc`).
/// На платформах без `RLIMIT_NOFILE` (Windows) — тихо ничего не делает.
fn raise_fd_limit() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| match rlimit::increase_nofile_limit(u64::MAX) {
        Ok(limit) => tracing::info!("лимит открытых файлов поднят до {limit}"),
        Err(e) => tracing::warn!("не удалось поднять лимит открытых файлов: {e}"),
    });
}

impl Engine {
    /// Запускает сессию движка.
    pub async fn start(config: EngineConfig) -> Result<Self> {
        raise_fd_limit();

        std::fs::create_dir_all(&config.download_dir).map_err(|e| Error::Start(e.to_string()))?;
        std::fs::create_dir_all(&config.state_dir).map_err(|e| Error::Start(e.to_string()))?;

        let opts = SessionOptions {
            persistence: Some(SessionPersistenceConfig::Json {
                folder: Some(config.state_dir.clone()),
            }),
            fastresume: true,
            listen_port_range: config.listen_port.map(|p| p..p.saturating_add(1)),
            ..Default::default()
        };

        let session = Session::new_with_opts(config.download_dir.clone(), opts)
            .await
            .map_err(|e| Error::Start(e.to_string()))?;

        Ok(Self {
            session,
            download_dir: config.download_dir,
        })
    }

    fn build_add(source: &Source) -> AddTorrent<'_> {
        match source {
            Source::Url(url) => AddTorrent::from_url(url.clone()),
            Source::TorrentBytes(bytes) => AddTorrent::from_bytes(bytes.clone()),
        }
    }

    /// Добавляет торрент. Возвращает его info-hash (hex).
    pub async fn add(&self, source: Source, params: AddParams) -> Result<String> {
        let opts = AddTorrentOptions {
            paused: params.paused,
            only_files: params.only_files,
            output_folder: params.output_folder,
            overwrite: true,
            ..Default::default()
        };

        let response = self
            .session
            .add_torrent(Self::build_add(&source), Some(opts))
            .await
            .map_err(|e| Error::Backend(e.to_string()))?;

        let hash = match response {
            AddTorrentResponse::Added(_, handle) => handle.info_hash().as_string(),
            AddTorrentResponse::AlreadyManaged(_, handle) => handle.info_hash().as_string(),
            AddTorrentResponse::ListOnly(list) => list.info_hash.as_string(),
        };
        Ok(hash)
    }

    /// Разбирает источник без запуска скачивания и возвращает список файлов.
    ///
    /// Для magnet без метаданных librqbit сначала докачивает metainfo из сети —
    /// ограничиваем ожидание таймаутом, чтобы UI не завис.
    pub async fn preview(&self, source: Source) -> Result<TorrentPreview> {
        let opts = AddTorrentOptions {
            list_only: true,
            ..Default::default()
        };
        let fut = self
            .session
            .add_torrent(Self::build_add(&source), Some(opts));
        let response = tokio::time::timeout(std::time::Duration::from_secs(90), fut)
            .await
            .map_err(|_| Error::Backend("превышено время получения списка файлов".into()))?
            .map_err(|e| Error::Backend(e.to_string()))?;

        let AddTorrentResponse::ListOnly(list) = response else {
            return Err(Error::InvalidSource(
                "не удалось получить список файлов".into(),
            ));
        };

        let mut files = Vec::new();
        let mut total = 0u64;
        let details = list
            .info
            .iter_file_details()
            .map_err(|e| Error::Backend(e.to_string()))?;
        for (index, detail) in details.enumerate() {
            let path = detail
                .filename
                .to_vec()
                .map_err(|e| Error::Backend(e.to_string()))?
                .join("/");
            total += detail.len;
            files.push(EngineFile {
                index,
                path,
                size: detail.len,
            });
        }

        Ok(TorrentPreview {
            hash: list.info_hash.as_string(),
            // Имя формирует вызывающая сторона (из заголовка раздачи rutracker),
            // чтобы не зависеть от внутреннего буферного типа librqbit.
            name: String::new(),
            files,
            total_size: total,
        })
    }

    /// Находит управляемый торрент по hex-хэшу.
    ///
    /// Итерируемся циклом `for` по `&mut *torrents`: обобщённые комбинаторы
    /// (`Iterator::find`, `map`) нельзя вызвать на `&mut dyn Iterator` — они
    /// требуют `Self: Sized`.
    fn find(&self, hash: &str) -> Option<(usize, Handle)> {
        self.session.with_torrents(|torrents| {
            for (id, handle) in &mut *torrents {
                if handle.info_hash().as_string() == hash {
                    return Some((id, handle.clone()));
                }
            }
            None
        })
    }

    /// Ставит торрент на паузу.
    pub async fn pause(&self, hash: &str) -> Result<()> {
        let (_, handle) = self.find(hash).ok_or(Error::NotFound)?;
        self.session
            .pause(&handle)
            .await
            .map_err(|e| Error::Backend(e.to_string()))
    }

    /// Возобновляет торрент.
    pub async fn resume(&self, hash: &str) -> Result<()> {
        let (_, handle) = self.find(hash).ok_or(Error::NotFound)?;
        self.session
            .unpause(&handle)
            .await
            .map_err(|e| Error::Backend(e.to_string()))
    }

    /// Удаляет торрент (опционально с файлами).
    pub async fn remove(&self, hash: &str, delete_files: bool) -> Result<()> {
        let (id, _) = self.find(hash).ok_or(Error::NotFound)?;
        self.session
            .delete(TorrentIdOrHash::Id(id), delete_files)
            .await
            .map_err(|e| Error::Backend(e.to_string()))
    }

    /// Снимок всех торрентов.
    pub fn torrents(&self) -> Vec<EngineTorrent> {
        let handles: Vec<Handle> = self.session.with_torrents(|torrents| {
            let mut handles = Vec::new();
            for (_, handle) in &mut *torrents {
                handles.push(handle.clone());
            }
            handles
        });
        handles
            .iter()
            .map(|handle| snapshot(handle, &self.download_dir))
            .collect()
    }
}

/// Список файлов из содержимого `.torrent`-файла — без сети и без запуска
/// сессии (используется отслеживанием версий раздач).
pub fn torrent_files(bytes: &[u8]) -> Result<Vec<EngineFile>> {
    Ok(parse_torrent(bytes)?.1)
}

/// Метаданные `.torrent`-файла: имя, info-hash (hex), суммарный размер и
/// список файлов. Для импорта сторонних торрентов.
pub fn torrent_meta(bytes: &[u8]) -> Result<TorrentMeta> {
    let (torrent, files) = parse_torrent(bytes)?;
    let name = torrent
        .info
        .name
        .as_ref()
        .map(|n| String::from_utf8_lossy(n).into_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "Без имени".to_owned());
    Ok(TorrentMeta {
        name,
        info_hash: torrent.info_hash.as_string(),
        total_size: files.iter().map(|f| f.size).sum(),
        files,
    })
}

/// Метаданные разобранного `.torrent`-файла.
#[derive(Debug, Clone)]
pub struct TorrentMeta {
    pub name: String,
    pub info_hash: String,
    pub total_size: u64,
    pub files: Vec<EngineFile>,
}

type ParsedTorrent = librqbit::TorrentMetaV1<librqbit::ByteBufOwned>;

fn parse_torrent(bytes: &[u8]) -> Result<(ParsedTorrent, Vec<EngineFile>)> {
    let torrent = librqbit::torrent_from_bytes::<librqbit::ByteBufOwned>(bytes)
        .map_err(|e| Error::InvalidSource(format!("не удалось разобрать .torrent: {e}")))?;
    let details = torrent
        .info
        .iter_file_details()
        .map_err(|e| Error::Backend(e.to_string()))?;

    let mut files = Vec::new();
    for (index, detail) in details.enumerate() {
        let path = detail
            .filename
            .to_vec()
            .map_err(|e| Error::Backend(e.to_string()))?
            .join("/");
        files.push(EngineFile {
            index,
            path,
            size: detail.len,
        });
    }
    Ok((torrent, files))
}

/// Скорость (байт/с) из сглаженной оценки librqbit (в MiB/с).
fn to_bps(mibps: f64) -> u64 {
    (mibps * 1024.0 * 1024.0).max(0.0) as u64
}

/// Строит снимок состояния торрента из данных librqbit.
fn snapshot(handle: &Handle, download_dir: &std::path::Path) -> EngineTorrent {
    let hash = handle.info_hash().as_string();
    let stats = handle.stats();

    let downloaded = stats.progress_bytes;
    let uploaded = stats.uploaded_bytes;
    let total = stats.total_bytes;

    // Скорости берём из встроенной сглаженной оценки librqbit — она не «моргает»
    // в 0 между завершением отдельных кусков (в отличие от дельты по кускам).
    let (dl_speed, up_speed) = match stats.live.as_ref() {
        Some(live) => (
            to_bps(live.download_speed.mbps),
            to_bps(live.upload_speed.mbps),
        ),
        None => (0, 0),
    };

    let peers = stats
        .live
        .as_ref()
        .map(|l| l.snapshot.peer_stats.live as u32)
        .unwrap_or(0);

    let state = match stats.state {
        TorrentStatsState::Paused => TorrentState::Paused,
        TorrentStatsState::Error => TorrentState::Error,
        TorrentStatsState::Initializing => TorrentState::Checking,
        TorrentStatsState::Live if stats.finished => TorrentState::Seeding,
        TorrentStatsState::Live => TorrentState::Downloading,
    };

    let progress = if total > 0 {
        (downloaded as f64 / total as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let eta_secs = if !stats.finished && dl_speed > 0 && total > downloaded {
        Some((total - downloaded) / dl_speed)
    } else {
        None
    };

    EngineTorrent {
        name: handle.name().unwrap_or_else(|| hash.clone()),
        hash,
        size: total,
        progress,
        downloaded,
        uploaded,
        dl_speed,
        up_speed,
        eta_secs,
        state,
        peers,
        finished: stats.finished,
        save_path: download_dir.display().to_string(),
        error: stats.error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mibps_to_bytes_per_second() {
        assert_eq!(to_bps(1.0), 1024 * 1024);
        assert_eq!(to_bps(0.0), 0);
        assert_eq!(to_bps(-1.0), 0);
    }

    #[test]
    fn state_str_roundtrip() {
        assert_eq!(TorrentState::Downloading.as_str(), "downloading");
        assert_eq!(TorrentState::Seeding.as_str(), "seeding");
    }
}

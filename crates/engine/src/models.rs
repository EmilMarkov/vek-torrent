//! Модели данных движка (не зависят от web/serde — маппинг делает vek-core).

/// Источник добавляемого торрента.
#[derive(Debug, Clone)]
pub enum Source {
    /// magnet- или http(s)-ссылка на `.torrent`.
    Url(String),
    /// Содержимое `.torrent`-файла.
    TorrentBytes(Vec<u8>),
}

/// Параметры добавления торрента.
#[derive(Debug, Clone, Default)]
pub struct AddParams {
    /// Каталог сохранения (иначе — каталог сессии по умолчанию).
    pub output_folder: Option<String>,
    /// Добавить остановленным.
    pub paused: bool,
    /// Индексы файлов для скачивания (None — все файлы).
    pub only_files: Option<Vec<usize>>,
}

/// Нормализованное состояние торрента.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TorrentState {
    /// Идёт скачивание.
    Downloading,
    /// Скачано, идёт раздача.
    Seeding,
    /// На паузе.
    Paused,
    /// Инициализация/проверка данных.
    Checking,
    /// Ошибка.
    Error,
    /// Неизвестно.
    Unknown,
}

impl TorrentState {
    /// Строковый код состояния (для UI/логов).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Downloading => "downloading",
            Self::Seeding => "seeding",
            Self::Paused => "paused",
            Self::Checking => "checking",
            Self::Error => "error",
            Self::Unknown => "unknown",
        }
    }
}

/// Снимок состояния торрента.
#[derive(Debug, Clone)]
pub struct EngineTorrent {
    /// Info-hash в hex.
    pub hash: String,
    pub name: String,
    /// Полный размер (выбранных файлов), байт.
    pub size: u64,
    /// Прогресс 0.0..=1.0.
    pub progress: f64,
    /// Скачано байт.
    pub downloaded: u64,
    /// Отдано байт.
    pub uploaded: u64,
    /// Скорость скачивания, байт/с.
    pub dl_speed: u64,
    /// Скорость отдачи, байт/с.
    pub up_speed: u64,
    /// Оставшееся время, секунды (None — неизвестно).
    pub eta_secs: Option<u64>,
    pub state: TorrentState,
    /// Число подключённых пиров.
    pub peers: u32,
    pub finished: bool,
    /// Каталог сохранения.
    pub save_path: String,
    /// Текст ошибки, если состояние Error.
    pub error: Option<String>,
}

/// Файл внутри торрента (для выбора при добавлении).
#[derive(Debug, Clone)]
pub struct EngineFile {
    /// Индекс файла в торренте.
    pub index: usize,
    /// Путь файла внутри раздачи.
    pub path: String,
    /// Размер файла, байт.
    pub size: u64,
}

/// Результат разбора источника без запуска скачивания (список файлов).
#[derive(Debug, Clone)]
pub struct TorrentPreview {
    pub hash: String,
    pub name: String,
    pub files: Vec<EngineFile>,
    pub total_size: u64,
}

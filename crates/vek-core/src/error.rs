//! Единый тип ошибки доменного слоя.

/// Ошибка операций VEK Torrent.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Ошибка клиента rutracker.
    #[error(transparent)]
    Rutracker(#[from] rutracker::Error),

    /// Ошибка встроенного торрент-движка.
    #[error(transparent)]
    Engine(#[from] engine::Error),

    /// Требуется ввод капчи при входе на rutracker.
    #[error("требуется ввод капчи")]
    CaptchaRequired(Box<rutracker::models::CaptchaChallenge>),

    /// Не заданы учётные данные rutracker.
    #[error("не заданы логин и пароль rutracker")]
    NoCredentials,

    /// Торрент-движок недоступен / не запустился.
    #[error("торрент-движок недоступен: {0}")]
    EngineUnavailable(String),

    /// Ошибка конфигурации.
    #[error("ошибка конфигурации: {0}")]
    Config(String),

    /// Ошибка ввода-вывода.
    #[error("ошибка ввода-вывода: {0}")]
    Io(#[from] std::io::Error),

    /// Ошибка сериализации конфигурации.
    #[error("ошибка сериализации: {0}")]
    Serde(#[from] serde_json::Error),
}

impl Error {
    /// Преобразует ошибку rutracker, вынося капчу в отдельный вариант,
    /// чтобы её было удобно обработать в UI/API.
    pub fn from_rutracker(err: rutracker::Error) -> Self {
        match err {
            rutracker::Error::CaptchaRequired(challenge) => Self::CaptchaRequired(challenge),
            other => Self::Rutracker(other),
        }
    }
}

/// Результат операций доменного слоя.
pub type Result<T> = std::result::Result<T, Error>;

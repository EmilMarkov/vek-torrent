//! Ошибки клиента qBittorrent.

/// Ошибка обращения к qBittorrent Web API.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Сетевая ошибка.
    #[error("сетевая ошибка qBittorrent: {0}")]
    Http(#[from] reqwest::Error),

    /// Некорректный базовый URL.
    #[error("некорректный адрес qBittorrent: {0}")]
    Url(String),

    /// Не удалось авторизоваться в Web API.
    #[error("не удалось авторизоваться в qBittorrent (неверные логин/пароль)")]
    AuthFailed,

    /// Требуется авторизация (403), но сессии нет.
    #[error("qBittorrent требует авторизацию")]
    Forbidden,

    /// Операция отклонена с ошибкой (например, повреждённый torrent).
    #[error("qBittorrent отклонил операцию: {0}")]
    OperationFailed(String),

    /// Неожиданный ответ (не удалось разобрать JSON).
    #[error("не удалось разобрать ответ qBittorrent: {0}")]
    Decode(String),

    /// Забанен по IP за слишком частые неудачные попытки входа.
    #[error("qBittorrent временно заблокировал доступ (слишком много попыток входа)")]
    Banned,
}

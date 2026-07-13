//! Ошибки клиента rutracker.

use crate::models::CaptchaChallenge;

/// Ошибка операции с трекером.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Сетевая ошибка (включая таймауты и ошибки TLS/прокси).
    #[error("сетевая ошибка: {0}")]
    Http(#[from] reqwest::Error),

    /// Требуется вход на трекер.
    #[error("требуется вход на rutracker")]
    NotAuthenticated,

    /// Сервер запросил капчу при входе.
    #[error("rutracker требует ввод капчи")]
    CaptchaRequired(Box<CaptchaChallenge>),

    /// Неверная пара логин/пароль.
    #[error("неверный логин или пароль")]
    BadCredentials,

    /// Доступ ограничен (бан, блокировка, требуется подтверждение).
    #[error("доступ ограничен: {0}")]
    AccessDenied(String),

    /// Не удалось разобрать страницу (изменилась разметка или пришла заглушка).
    #[error("не удалось разобрать страницу: {0}")]
    Parse(String),

    /// Некорректный URL (зеркало или прокси).
    #[error("некорректный URL: {0}")]
    Url(String),

    /// Ошибка файловой системы (персистентность куков).
    #[error("ошибка ввода-вывода: {0}")]
    Io(#[from] std::io::Error),

    /// Ошибка хранилища куков.
    #[error("ошибка хранилища куков: {0}")]
    CookieStore(String),
}

impl Error {
    /// Утилита для ошибок парсинга с контекстом.
    pub(crate) fn parse(context: impl Into<String>) -> Self {
        Self::Parse(context.into())
    }

    /// Похожа ли ошибка на недоступность или блокировку зеркала —
    /// кандидат на автоматическое переключение на другое зеркало.
    pub fn is_mirror_unreachable(&self) -> bool {
        match self {
            Self::Http(e) => {
                e.is_connect()
                    // Обрыв при чтении тела и redirect-петли — типичные
                    // проявления DPI-блокировок наряду с connect/timeout.
                    || e.is_timeout()
                    || e.is_body()
                    || e.is_decode()
                    || e.is_redirect()
                    || matches!(
                        e.status(),
                        Some(s) if s.as_u16() == 403 || s.as_u16() == 451 || s.is_server_error()
                    )
            }
            _ => false,
        }
    }
}

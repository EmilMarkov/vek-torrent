//! Ошибка команд, сериализуемая во фронтенд.

use serde::Serialize;
use vek_core::Error as CoreError;

/// Структурированная ошибка команды Tauri.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    /// Машиночитаемый код (`captcha_required`, `qbit_unavailable`, …).
    pub code: String,
    /// Человекочитаемое сообщение.
    pub message: String,
}

impl CommandError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }
}

impl From<CoreError> for CommandError {
    fn from(err: CoreError) -> Self {
        let code = match &err {
            CoreError::CaptchaRequired(_) => "captcha_required",
            CoreError::NoCredentials => "no_credentials",
            CoreError::Rutracker(rutracker::Error::NotAuthenticated) => "not_authenticated",
            CoreError::Rutracker(rutracker::Error::BadCredentials) => "bad_credentials",
            CoreError::Engine(_) | CoreError::EngineUnavailable(_) => "engine_error",
            CoreError::Config(_) => "invalid_config",
            _ => "error",
        };
        Self::new(code, err.to_string())
    }
}

impl From<std::io::Error> for CommandError {
    fn from(err: std::io::Error) -> Self {
        Self::new("io_error", err.to_string())
    }
}

/// Результат команды.
pub type CommandResult<T> = std::result::Result<T, CommandError>;

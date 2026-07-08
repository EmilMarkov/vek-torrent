//! Ошибки внешнего API и их отображение в HTTP-ответы.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;
use vek_core::Error as CoreError;

/// Ошибка HTTP-обработчика.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
    /// Машиночитаемый код ошибки.
    code: &'static str,
}

/// Тело ответа с ошибкой.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    /// Машиночитаемый код (`unauthorized`, `not_authenticated`, …).
    pub code: String,
    /// Человекочитаемое описание.
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "требуется корректный Bearer-токен",
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorBody {
            code: self.code.to_owned(),
            message: self.message,
        };
        (self.status, Json(body)).into_response()
    }
}

impl From<CoreError> for ApiError {
    fn from(err: CoreError) -> Self {
        let (status, code) = match &err {
            CoreError::CaptchaRequired(_) => (StatusCode::CONFLICT, "captcha_required"),
            CoreError::NoCredentials => (StatusCode::BAD_REQUEST, "no_credentials"),
            CoreError::Rutracker(rutracker::Error::NotAuthenticated) => {
                (StatusCode::UNAUTHORIZED, "not_authenticated")
            }
            CoreError::Rutracker(rutracker::Error::BadCredentials) => {
                (StatusCode::UNAUTHORIZED, "bad_credentials")
            }
            CoreError::Engine(_) | CoreError::EngineUnavailable(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, "engine_unavailable")
            }
            CoreError::Config(_) => (StatusCode::BAD_REQUEST, "invalid_config"),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };
        Self::new(status, code, err.to_string())
    }
}

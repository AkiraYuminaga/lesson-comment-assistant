use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Gemini API error: {0}")]
    GeminiApiError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Upstream AI service is busy")]
    UpstreamBusy,

    #[error("Internal server error")]
    InternalError,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::GeminiApiError(_)
            | AppError::ConfigError(_)
            | AppError::EncryptionError(_) => {
                // Log the actual error internally but return generic message
                tracing::error!("Internal error: {}", self);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An internal error occurred".to_string(),
                )
            }
            AppError::UpstreamBusy => (
                StatusCode::SERVICE_UNAVAILABLE,
                "AI service is busy. Please try again shortly.".to_string(),
            ),
            AppError::InternalError => {
                tracing::error!("Internal error: {}", self);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An internal error occurred".to_string(),
                )
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

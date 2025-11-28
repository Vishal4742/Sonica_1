use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Audio processing error: {0}")]
    Audio(String),

    #[error("FFmpeg error: {0}")]
    Ffmpeg(String),

    #[error("Fingerprint error: {0}")]
    Fingerprint(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("External API error: {0}")]
    External(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::Io(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::Json(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            AppError::Audio(e) => (StatusCode::BAD_REQUEST, e),
            AppError::Ffmpeg(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
            AppError::Fingerprint(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
            AppError::NotFound(e) => (StatusCode::NOT_FOUND, e),
            AppError::InvalidRequest(e) => (StatusCode::BAD_REQUEST, e),
            AppError::External(e) => (StatusCode::BAD_GATEWAY, e),
            AppError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
        };

        let body = Json(json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

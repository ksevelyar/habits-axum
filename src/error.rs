use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Debug)]
pub enum AppError {
    Unauthorized,
    NotFound(String),
    Validation(Vec<FieldError>),
    BadRequest(String),
    ApplicationError(String),
    ConfigError(String),
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::RowNotFound => AppError::NotFound("record not found".into()),
            error => AppError::ApplicationError(error.to_string()),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Unauthorized => write!(formatter, "unauthorized"),
            AppError::NotFound(message) => write!(formatter, "not found: {message}"),
            AppError::Validation(fields) => write!(formatter, "validation failed: {fields:?}"),
            AppError::BadRequest(message) => write!(formatter, "bad request: {message}"),
            AppError::ApplicationError(message) => write!(formatter, "internal error: {message}"),
            AppError::ConfigError(message) => write!(formatter, "configuration error: {message}"),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, json!({"error": "unauthorized"})),
            AppError::NotFound(message) => (StatusCode::NOT_FOUND, json!({"error": message})),
            AppError::Validation(fields) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({"error": "validation failed", "fields": fields}),
            ),
            AppError::BadRequest(message) => (StatusCode::BAD_REQUEST, json!({"error": message})),
            AppError::ApplicationError(_) => (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": "internal error"})),
            AppError::ConfigError(message) => (StatusCode::SERVICE_UNAVAILABLE, json!({"error": message})),
        };
        (status, Json(body)).into_response()
    }
}

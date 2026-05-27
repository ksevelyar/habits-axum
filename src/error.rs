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
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, json!({"error": "unauthorized"})),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, json!({"error": msg})),
            AppError::Validation(fields) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({"error": "validation failed", "fields": fields}),
            ),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, json!({"error": msg})),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": msg})),
        };
        (status, Json(body)).into_response()
    }
}

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use std::sync::Arc;

use crate::AppState;
use crate::authentication::authenticate_cookie;
use crate::error::{AppError, FieldError};
use crate::tasks::Task;

#[derive(Deserialize)]
pub struct TaskPayload {
    pub name: Option<String>,
    pub cron: Option<String>,
    pub active: Option<bool>,
}

pub async fn list(State(state): State<Arc<AppState>>, cookie_jar: CookieJar) -> Result<Json<Vec<Task>>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::tasks::list_by_user_id(&state.pool, user.id)
        .await
        .map_err(|err| {
            tracing::error!("{err}");
            AppError::BadRequest("database error".into())
        })
        .map(Json)
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Json(body): Json<TaskPayload>,
) -> Result<(StatusCode, Json<Task>), AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;

    let name = body.name.ok_or(AppError::Validation(vec![FieldError {
        field: "name".into(),
        message: "name is required".into(),
    }]))?;
    let cron = body.cron.ok_or(AppError::Validation(vec![FieldError {
        field: "cron".into(),
        message: "cron is required".into(),
    }]))?;

    crate::tasks::create(&state.pool, user.id, &name, &cron, body.active.unwrap_or(false))
        .await
        .map(|task| (StatusCode::CREATED, Json(task)))
}

pub async fn show(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(task_id): Path<i64>,
) -> Result<Json<Task>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::tasks::find_by_id(&state.pool, user.id, task_id).await.map(Json)
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(task_id): Path<i64>,
    Json(body): Json<TaskPayload>,
) -> Result<Json<Task>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::tasks::update(&state.pool, user.id, task_id, body.name, body.cron, body.active)
        .await
        .map(Json)
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(task_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::tasks::delete(&state.pool, user.id, task_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

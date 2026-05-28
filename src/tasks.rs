use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::AppError;
use crate::users::authenticate_user;

#[derive(Deserialize)]
pub struct TaskPayload {
    pub name: Option<String>,
    pub cron: Option<String>,
    pub active: Option<bool>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Task {
    pub id: i64,

    pub name: Option<String>,
    pub active: bool,
    pub cron: Option<String>,

    pub user_id: i64,

    pub inserted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn list(
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
) -> Result<Json<Vec<Task>>, AppError> {
    let current_user = authenticate_user(&pool, &cookie_jar).await?;

    let tasks = sqlx::query_as::<_, Task>(
        r#"
        SELECT *
        FROM tasks
        WHERE user_id = $1
        ORDER BY id DESC
        "#,
    )
    .bind(current_user.id)
    .fetch_all(&pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::BadRequest("database error".into())
    })?;

    Ok(Json(tasks))
}

pub async fn list_by_user_id(pool: &PgPool, user_id: i64) -> Result<Vec<Task>, sqlx::Error> {
    sqlx::query_as::<_, Task>(
        r#"
        SELECT *
        FROM tasks
        WHERE user_id = $1
        ORDER BY id DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn create(
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
    Json(data): Json<TaskPayload>,
) -> Result<(StatusCode, Json<Task>), AppError> {
    let current_user = authenticate_user(&pool, &cookie_jar).await?;

    let task = sqlx::query_as::<_, Task>(
        r#"
        INSERT INTO tasks (
            user_id,
            name,
            cron,
            active
        )
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(current_user.id)
    .bind(data.name)
    .bind(data.cron)
    .bind(data.active.unwrap_or(false))
    .fetch_one(&pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::BadRequest("database error".into())
    })?;

    Ok((StatusCode::CREATED, Json(task)))
}

pub async fn show(
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
    Path(task_id): Path<i64>,
) -> Result<Json<Task>, AppError> {
    let current_user = authenticate_user(&pool, &cookie_jar).await?;
    let task = find_task(&pool, current_user.id, task_id).await?;

    Ok(Json(task))
}

async fn find_task(pool: &PgPool, user_id: i64, task_id: i64) -> Result<Task, AppError> {
    sqlx::query_as::<_, Task>(
        r#"
        SELECT *
        FROM tasks
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(task_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::NotFound("task not found".into())
    })
}

pub async fn update(
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
    Path(task_id): Path<i64>,
    Json(data): Json<TaskPayload>,
) -> Result<Json<Task>, AppError> {
    let current_user = authenticate_user(&pool, &cookie_jar).await?;

    let task = sqlx::query_as::<_, Task>(
        r#"
        UPDATE tasks
        SET
            name = COALESCE($1, name),
            cron = COALESCE($2, cron),
            active = COALESCE($3, active),
            updated_at = NOW()
        WHERE id = $4 AND user_id = $5
        RETURNING *
        "#,
    )
    .bind(data.name)
    .bind(data.cron)
    .bind(data.active)
    .bind(task_id)
    .bind(current_user.id)
    .fetch_one(&pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::BadRequest("database error".into())
    })?;

    Ok(Json(task))
}

pub async fn delete(
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
    Path(task_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let current_user = authenticate_user(&pool, &cookie_jar).await?;

    sqlx::query(
        r#"
        DELETE FROM tasks
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(task_id)
    .bind(current_user.id)
    .execute(&pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::BadRequest("database error".into())
    })?;

    Ok(StatusCode::NO_CONTENT)
}

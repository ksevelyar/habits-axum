use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use tracing::Level;

use crate::error::AppError;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Task {
    pub id: i64,

    pub name: String,
    pub active: bool,
    pub cron: String,

    pub user_id: i64,

    pub inserted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn list_by_user_id(pool: &PgPool, user_id: i64) -> Result<Vec<Task>, AppError> {
    Ok(sqlx::query_as::<_, Task>(
        r#"
        SELECT *
        FROM tasks
        WHERE user_id = $1
        ORDER BY id DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn find_by_id(pool: &PgPool, user_id: i64, task_id: i64) -> Result<Task, AppError> {
    sqlx::query_as::<_, Task>(
        r#"
        SELECT *
        FROM tasks
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(task_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound("task not found".into()))
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn create(pool: &PgPool, user_id: i64, name: &str, cron: &str, active: bool) -> Result<Task, AppError> {
    Ok(sqlx::query_as::<_, Task>(
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
    .bind(user_id)
    .bind(name)
    .bind(cron)
    .bind(active)
    .fetch_one(pool)
    .await?)
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn update(
    pool: &PgPool,
    user_id: i64,
    task_id: i64,
    name: Option<String>,
    cron: Option<String>,
    active: Option<bool>,
) -> Result<Task, AppError> {
    Ok(sqlx::query_as::<_, Task>(
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
    .bind(name)
    .bind(cron)
    .bind(active)
    .bind(task_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?)
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn delete(pool: &PgPool, user_id: i64, task_id: i64) -> Result<(), AppError> {
    let deletion = sqlx::query(
        r#"
        DELETE FROM tasks
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(task_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    if deletion.rows_affected() == 0 {
        return Err(AppError::NotFound("task not found".into()));
    }
    Ok(())
}

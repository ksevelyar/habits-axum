use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use cron::Schedule;
use serde::Serialize;
use sqlx::PgPool;
use std::str::FromStr;

use crate::error::AppError;
use crate::users::User;

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
    .fetch_one(pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::NotFound("task not found".into())
    })
}

pub async fn create(
    pool: &PgPool,
    user_id: i64,
    name: &str,
    cron: &str,
    active: bool,
) -> Result<Task, AppError> {
    sqlx::query_as::<_, Task>(
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
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::BadRequest("database error".into())
    })
}

pub async fn update(
    pool: &PgPool,
    user_id: i64,
    task_id: i64,
    name: Option<String>,
    cron: Option<String>,
    active: Option<bool>,
) -> Result<Task, AppError> {
    sqlx::query_as::<_, Task>(
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
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::BadRequest("database error".into())
    })
}

pub async fn delete(pool: &PgPool, user_id: i64, task_id: i64) -> Result<(), AppError> {
    sqlx::query(
        r#"
        DELETE FROM tasks
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(task_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::BadRequest("database error".into())
    })?;

    Ok(())
}

pub async fn eval_next_notification(pool: &PgPool, user: &User) -> Option<(Task, DateTime<Utc>)> {
    let tasks = match list_by_user_id(pool, user.id).await {
        Ok(tasks) => tasks,
        Err(e) => {
            tracing::error!("{e}");
            return None;
        }
    };
    let tz: Tz = user.timezone.parse().ok()?;

    tasks
        .into_iter()
        .filter(|task| task.active)
        .filter_map(|task| {
            let schedule = Schedule::from_str(&task.cron).ok()?;
            let next_run = schedule.upcoming(tz).next()?;
            Some((task, next_run.with_timezone(&Utc)))
        })
        .min_by_key(|(_, next_run)| *next_run)
}

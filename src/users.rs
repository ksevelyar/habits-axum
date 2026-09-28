use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::env;
use tracing::Level;

use crate::error::{AppError, FieldError};

#[derive(Debug, Deserialize)]
pub struct CreatePayload {
    pub email: String,
    pub handle: String,
    pub password: String,
    pub timezone: String,
}

#[derive(Serialize, sqlx::FromRow, Debug, Clone)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub timezone: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct BackendUser {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
}

#[derive(Serialize)]
pub struct DeviceTokenResponse {
    pub device_id: String,
    pub device_name: String,
    pub token: String,
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<BackendUser, AppError> {
    sqlx::query_as::<_, BackendUser>("SELECT * FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::Unauthorized)
}

pub async fn find_by_id(pool: &PgPool, user_id: i64) -> Result<Option<User>, AppError> {
    Ok(
        sqlx::query_as::<_, User>("SELECT id, email, timezone FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn set_dev_password(pool: &PgPool) {
    if let Ok(dev_password) = env::var("DEV_PASSWORD") {
        let hash = crate::authentication::hash(&dev_password).unwrap();
        sqlx::query(
            "INSERT INTO users (handle, email, password_hash, inserted_at, updated_at)
             VALUES ($1, $2, $3, NOW(), NOW())
             ON CONFLICT (email) DO UPDATE SET password_hash = $3",
        )
        .bind("ksevelyar")
        .bind("ksevelyar@gmail.com")
        .bind(&hash)
        .execute(pool)
        .await
        .expect("Failed to seed dev user");
        println!("🐗 Seeded dev user: ksevelyar@gmail.com");
    }
}

#[tracing::instrument(skip(pool, payload), err(level = Level::ERROR))]
pub async fn create(pool: &PgPool, payload: CreatePayload) -> Result<User, AppError> {
    let hashed_password = crate::authentication::hash(&payload.password)?;
    payload.timezone.parse::<chrono_tz::Tz>().map_err(|_| {
        AppError::Validation(vec![FieldError {
            field: "timezone".into(),
            message: "unknown timezone".into(),
        }])
    })?;

    sqlx::query_as::<_, User>(
        "INSERT INTO users (email, password_hash, timezone, handle)
         VALUES ($1, $2, $3, $4)
         RETURNING id, email, timezone",
    )
    .bind(payload.email)
    .bind(hashed_password)
    .bind(payload.timezone)
    .bind(payload.handle)
    .fetch_one(pool)
    .await
    .map_err(|error| match error {
        sqlx::Error::Database(database_error) if database_error.is_unique_violation() => {
            AppError::Validation(vec![FieldError {
                field: "email".into(),
                message: "email is already taken".into(),
            }])
        }
        error => error.into(),
    })
}

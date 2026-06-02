use axum::response::IntoResponse;
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;

use crate::authentication::{authenticate_cookie, build_cookie, encode_device_jwt, encode_jwt};
use crate::error::AppError;
use std::env;
use uuid::Uuid;

#[derive(Serialize, sqlx::FromRow, Debug, Clone)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub timezone: String,
}

#[derive(Deserialize)]
pub struct CreateSessionPayload {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
pub struct CreateUserPayload {
    pub email: String,
    pub handle: String,
    pub password: String,
    pub timezone: String,
}

#[derive(Deserialize)]
pub struct CreateDevicePayload {
    pub device_name: String,
}

#[derive(Serialize)]
pub struct DeviceTokenResponse {
    pub device_id: String,
    pub device_name: String,
    pub token: String,
}

pub async fn current(
    State(state): State<Arc<crate::AppState>>,
    cookie_jar: CookieJar,
) -> Result<Json<User>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    Ok(Json(user))
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct BackendUser {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
}

pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<BackendUser, sqlx::Error> {
    sqlx::query_as::<_, BackendUser>("SELECT * FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
}

pub async fn create_session(
    State(state): State<Arc<crate::AppState>>,
    cookie_jar: CookieJar,
    Json(user_data): Json<CreateSessionPayload>,
) -> impl IntoResponse {
    let user = find_by_email(&state.pool, &user_data.email)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let authenticated =
        crate::authentication::verify(&user_data.password, &user.password_hash).unwrap_or(false);
    if !authenticated {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let jwt_token = encode_jwt(user.email).map_err(|_| StatusCode::UNAUTHORIZED)?;
    Ok((
        StatusCode::CREATED,
        cookie_jar.add(build_cookie("jwt", jwt_token)),
    ))
}

pub async fn create(
    State(state): State<Arc<crate::AppState>>,
    Json(payload): Json<CreateUserPayload>,
) -> Result<(StatusCode, Json<User>), StatusCode> {
    let hashed_password = crate::authentication::hash(&payload.password)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let _valid_timezone: chrono_tz::Tz = payload.timezone.parse().map_err(|err| {
        tracing::error!("{err}");
        StatusCode::BAD_REQUEST
    })?;
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (email, password_hash, timezone, handle) 
         VALUES ($1, $2, $3, $4)
         RETURNING id, email, timezone",
    )
    .bind(payload.email)
    .bind(hashed_password)
    .bind(payload.timezone)
    .bind(payload.handle)
    .fetch_one(&state.pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        StatusCode::BAD_REQUEST
    })?;
    Ok((StatusCode::CREATED, Json(user)))
}

pub async fn create_device(
    State(state): State<Arc<crate::AppState>>,
    cookie_jar: CookieJar,
    Json(payload): Json<CreateDevicePayload>,
) -> Result<(StatusCode, Json<DeviceTokenResponse>), AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let device_id = Uuid::new_v4().to_string();
    let device_name = payload.device_name;
    let token = encode_device_jwt(user.email, device_id.clone(), device_name.clone())
        .map_err(|_| AppError::Internal("failed to generate token".into()))?;
    Ok((
        StatusCode::CREATED,
        Json(DeviceTokenResponse {
            device_id,
            device_name,
            token,
        }),
    ))
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

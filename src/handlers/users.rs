use axum::response::IntoResponse;
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use sqlx::Error as SqlxError;
use std::sync::Arc;
use tracing::Level;
use uuid::Uuid;

use crate::AppState;
use crate::authentication::authenticate_cookie;
use crate::authentication::{build_cookie, encode_device_jwt, encode_jwt};
use crate::error::AppError;
use crate::users::{DeviceTokenResponse, User};

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

pub async fn show_current_user(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
) -> Result<Json<User>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    Ok(Json(user))
}

#[tracing::instrument(skip(state, cookie_jar, user_data), err(level = Level::ERROR))]
pub async fn create_session(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Json(user_data): Json<CreateSessionPayload>,
) -> Result<(StatusCode, impl IntoResponse), AppError> {
    let user = match crate::users::find_by_email(&state.pool, &user_data.email).await {
        Ok(user) => user,
        Err(SqlxError::RowNotFound) => return Err(AppError::Unauthorized),
        Err(error) => return Err(error.into()),
    };
    let authenticated = crate::authentication::verify(&user_data.password, &user.password_hash).unwrap_or(false);
    if !authenticated {
        return Err(AppError::Unauthorized);
    }

    let jwt_token = encode_jwt(user.email)?;
    Ok((StatusCode::CREATED, cookie_jar.add(build_cookie("jwt", jwt_token))))
}

#[tracing::instrument(skip(state, payload), err(level = Level::ERROR))]
pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateUserPayload>,
) -> Result<(StatusCode, Json<User>), AppError> {
    let hashed_password = crate::authentication::hash(&payload.password)?;
    let _valid_timezone: chrono_tz::Tz = payload.timezone.parse().map_err(|_| {
        AppError::Validation(vec![crate::error::FieldError {
            field: "timezone".into(),
            message: "unknown timezone".into(),
        }])
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
    .map_err(|error| match error {
        SqlxError::Database(database_error) if database_error.is_unique_violation() => {
            AppError::Validation(vec![crate::error::FieldError {
                field: "email".into(),
                message: "email is already taken".into(),
            }])
        }
        error => error.into(),
    })?;

    Ok((StatusCode::CREATED, Json(user)))
}

#[tracing::instrument(skip(state, cookie_jar, payload), err(level = Level::ERROR))]
pub async fn create_device(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Json(payload): Json<CreateDevicePayload>,
) -> Result<(StatusCode, Json<DeviceTokenResponse>), AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let device_id = Uuid::new_v4().to_string();
    let device_name = payload.device_name;
    let token = encode_device_jwt(user.email, device_id.clone(), device_name.clone())?;
    Ok((
        StatusCode::CREATED,
        Json(DeviceTokenResponse {
            device_id,
            device_name,
            token,
        }),
    ))
}

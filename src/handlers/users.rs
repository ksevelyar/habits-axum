use axum::response::IntoResponse;
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use std::sync::Arc;
use tracing::Level;
use uuid::Uuid;

use crate::AppState;
use crate::authentication::authenticate_cookie;
use crate::authentication::{build_cookie, encode_device_jwt, encode_jwt};
use crate::error::AppError;
use crate::users::{CreatePayload, DeviceTokenResponse, User};

#[derive(Deserialize)]
pub struct CreateSessionPayload {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
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
    let user = crate::users::find_by_email(&state.pool, &user_data.email).await?;
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
    Json(payload): Json<CreatePayload>,
) -> Result<(StatusCode, Json<User>), AppError> {
    let user = crate::users::create(&state.pool, payload).await?;

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

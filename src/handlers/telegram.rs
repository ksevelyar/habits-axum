use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum_extra::extract::cookie::CookieJar;
use serde_json::{Value, json};
use std::sync::Arc;

use crate::AppState;
use crate::authentication::authenticate_cookie;
use crate::error::AppError;
use crate::telegram;

pub async fn link(State(state): State<Arc<AppState>>, cookie_jar: CookieJar) -> Result<Json<Value>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    if state.telegram.is_none() {
        return Err(AppError::ConfigError("telegram is not configured".into()));
    }

    let linked = telegram::find_chat_id(&state.pool, user.id).await?.is_some();

    Ok(Json(json!({
        "linked": linked,
    })))
}

pub async fn request_link(State(state): State<Arc<AppState>>, cookie_jar: CookieJar) -> Result<Json<Value>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let Some(telegram_state) = &state.telegram else {
        return Err(AppError::ConfigError("telegram is not configured".into()));
    };

    let code = telegram::create_link_code(&state.pool, user.id).await?;

    Ok(Json(json!({
        "url": telegram::build_bot_link(telegram_state, &code),
    })))
}

pub async fn unlink(State(state): State<Arc<AppState>>, cookie_jar: CookieJar) -> Result<StatusCode, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    if state.telegram.is_none() {
        return Err(AppError::ConfigError("telegram is not configured".into()));
    }

    telegram::clear_telegram_chat_id(&state.pool, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use crate::AppState;
use crate::authentication::authenticate_cookie;
use crate::chains::{Chain, ChainAggregate, ChainType, CreateChainInput, UpdateChainInput};
use crate::error::{AppError, FieldError};

#[derive(Deserialize)]
pub struct CreateChainPayload {
    pub active: bool,
    pub aggregate: ChainAggregate,
    pub description: Option<String>,
    pub name: String,
    pub order: Option<i32>,
    pub r#type: ChainType,
}

#[derive(Deserialize)]
pub struct UpdateChainPayload {
    pub active: Option<bool>,
    pub aggregate: Option<ChainAggregate>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub order: Option<i32>,
    pub r#type: Option<ChainType>,
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
) -> Result<Json<Vec<Chain>>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::chains::list_by_user_id(&state.pool, user.id)
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
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<Chain>), AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;

    let data: CreateChainPayload = serde_path_to_error::deserialize(body).map_err(|err| {
        AppError::Validation(vec![FieldError {
            field: err.path().to_string(),
            message: err.to_string(),
        }])
    })?;

    crate::chains::create(
        &state.pool,
        CreateChainInput {
            user_id: user.id,
            active: data.active,
            name: data.name,
            r#type: data.r#type,
            aggregate: data.aggregate,
            description: data.description,
            order: data.order,
        },
    )
    .await
    .map(|chain| (StatusCode::CREATED, Json(chain)))
}

pub async fn show(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(chain_id): Path<i64>,
) -> Result<Json<Chain>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::chains::find_by_id(&state.pool, user.id, chain_id)
        .await
        .map(Json)
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(chain_id): Path<i64>,
    Json(body): Json<Value>,
) -> Result<Json<Chain>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;

    let data: UpdateChainPayload = serde_path_to_error::deserialize(body).map_err(|err| {
        AppError::Validation(vec![FieldError {
            field: err.path().to_string(),
            message: err.to_string(),
        }])
    })?;

    crate::chains::update(
        &state.pool,
        UpdateChainInput {
            user_id: user.id,
            chain_id,
            active: data.active,
            name: data.name,
            r#type: data.r#type,
            aggregate: data.aggregate,
            description: data.description,
            order: data.order,
        },
    )
    .await
    .map(Json)
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(chain_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::chains::delete(&state.pool, user.id, chain_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

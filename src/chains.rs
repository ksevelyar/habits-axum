use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Type};

use std::sync::Arc;

use crate::authentication::authenticate_cookie;
use crate::error::{AppError, FieldError};

#[derive(Debug, Clone, Copy, PartialEq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "chain_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ChainType {
    Integer,
    Float,
    Boolean,
}

#[derive(Debug, Clone, Copy, PartialEq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "chain_aggregate", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ChainAggregate {
    Sum,
    Avg,
}

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

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Chain {
    pub id: i64,
    pub user_id: i64,

    pub active: bool,
    pub name: String,
    pub r#type: ChainType,
    pub aggregate: ChainAggregate,

    pub description: Option<String>,
    pub order: Option<i32>,

    pub inserted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn list(
    State(state): State<Arc<crate::AppState>>,
    cookie_jar: CookieJar,
) -> Result<Json<Vec<Chain>>, AppError> {
    let current_user = authenticate_cookie(&state.pool, &cookie_jar).await?;

    let chains = sqlx::query_as::<_, Chain>(
        r#"
        SELECT *
        FROM chains
        WHERE user_id = $1
        ORDER BY id DESC
        "#,
    )
    .bind(current_user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::BadRequest("database error".into())
    })?;

    Ok(Json(chains))
}

pub async fn create(
    State(state): State<Arc<crate::AppState>>,
    cookie_jar: CookieJar,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<Chain>), AppError> {
    let current_user = authenticate_cookie(&state.pool, &cookie_jar).await?;

    let data: CreateChainPayload = serde_path_to_error::deserialize(body).map_err(|err| {
        AppError::Validation(vec![FieldError {
            field: err.path().to_string(),
            message: err.to_string(),
        }])
    })?;

    let chain = sqlx::query_as::<_, Chain>(
        r#"
        INSERT INTO chains (
            user_id,
            active,
            name,
            type,
            aggregate,
            description,
            "order"
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(current_user.id)
    .bind(data.active)
    .bind(data.name)
    .bind(data.r#type)
    .bind(data.aggregate)
    .bind(data.description)
    .bind(data.order)
    .fetch_one(&state.pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::BadRequest("database error".into())
    })?;

    Ok((StatusCode::CREATED, Json(chain)))
}

pub async fn show(
    State(state): State<Arc<crate::AppState>>,
    cookie_jar: CookieJar,
    Path(chain_id): Path<i64>,
) -> Result<Json<Chain>, AppError> {
    let current_user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let chain = find_chain(&state.pool, current_user.id, chain_id).await?;

    Ok(Json(chain))
}

async fn find_chain(pool: &PgPool, user_id: i64, chain_id: i64) -> Result<Chain, AppError> {
    sqlx::query_as::<_, Chain>(
        r#"
        SELECT *
        FROM chains
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(chain_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::NotFound("chain not found".into())
    })
}

pub async fn update(
    State(state): State<Arc<crate::AppState>>,
    cookie_jar: CookieJar,
    Path(chain_id): Path<i64>,
    Json(body): Json<Value>,
) -> Result<Json<Chain>, AppError> {
    let current_user = authenticate_cookie(&state.pool, &cookie_jar).await?;

    let data: UpdateChainPayload = serde_path_to_error::deserialize(body).map_err(|err| {
        AppError::Validation(vec![FieldError {
            field: err.path().to_string(),
            message: err.to_string(),
        }])
    })?;

    let chain = sqlx::query_as::<_, Chain>(
        r#"
        UPDATE chains
        SET
            active = COALESCE($1, active),
            name = COALESCE($2, name),
            type = COALESCE($3, type),
            aggregate = COALESCE($4, aggregate),
            description = COALESCE($5, description),
            "order" = COALESCE($6, "order"),
            updated_at = NOW()
        WHERE id = $7 AND user_id = $8
        RETURNING *
        "#,
    )
    .bind(data.active)
    .bind(data.name)
    .bind(data.r#type)
    .bind(data.aggregate)
    .bind(data.description)
    .bind(data.order)
    .bind(chain_id)
    .bind(current_user.id)
    .fetch_one(&state.pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::BadRequest("database error".into())
    })?;

    Ok(Json(chain))
}

pub async fn delete(
    State(state): State<Arc<crate::AppState>>,
    cookie_jar: CookieJar,
    Path(chain_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let current_user = authenticate_cookie(&state.pool, &cookie_jar).await?;

    sqlx::query(
        r#"
        DELETE FROM chains
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(chain_id)
    .bind(current_user.id)
    .execute(&state.pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::BadRequest("database error".into())
    })?;

    Ok(StatusCode::NO_CONTENT)
}

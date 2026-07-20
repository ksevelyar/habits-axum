use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Type};

use crate::error::AppError;

#[derive(Debug)]
pub struct CreateChainInput {
    pub user_id: i64,
    pub active: bool,
    pub name: String,
    pub r#type: ChainType,
    pub aggregate: ChainAggregate,
    pub description: Option<String>,
    pub order: Option<i32>,
}

#[derive(Debug)]
pub struct UpdateChainInput {
    pub user_id: i64,
    pub chain_id: i64,
    pub active: Option<bool>,
    pub name: Option<String>,
    pub r#type: Option<ChainType>,
    pub aggregate: Option<ChainAggregate>,
    pub description: Option<String>,
    pub order: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "chain_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ChainType {
    Integer,
    Float,
    Boolean,
    Time,
}

#[derive(Debug, Clone, Copy, PartialEq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "chain_aggregate", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ChainAggregate {
    Sum,
    Avg,
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

pub async fn list_by_user_id(pool: &PgPool, user_id: i64) -> Result<Vec<Chain>, sqlx::Error> {
    sqlx::query_as::<_, Chain>(
        r#"
        SELECT *
        FROM chains
        WHERE user_id = $1
        ORDER BY id DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, user_id: i64, chain_id: i64) -> Result<Chain, AppError> {
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

pub async fn create(pool: &PgPool, payload: CreateChainInput) -> Result<Chain, AppError> {
    sqlx::query_as::<_, Chain>(
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
    .bind(payload.user_id)
    .bind(payload.active)
    .bind(&payload.name)
    .bind(payload.r#type)
    .bind(payload.aggregate)
    .bind(payload.description)
    .bind(payload.order)
    .fetch_one(pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::BadRequest("database error".into())
    })
}

pub async fn update(pool: &PgPool, payload: UpdateChainInput) -> Result<Chain, AppError> {
    sqlx::query_as::<_, Chain>(
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
    .bind(payload.active)
    .bind(payload.name)
    .bind(payload.r#type)
    .bind(payload.aggregate)
    .bind(payload.description)
    .bind(payload.order)
    .bind(payload.chain_id)
    .bind(payload.user_id)
    .fetch_one(pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::BadRequest("database error".into())
    })
}

pub async fn delete(pool: &PgPool, user_id: i64, chain_id: i64) -> Result<(), AppError> {
    sqlx::query(
        r#"
        DELETE FROM chains
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(chain_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::BadRequest("database error".into())
    })?;

    Ok(())
}

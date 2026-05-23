use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Type};

use crate::users::{User, decode_jwt};

#[derive(Debug, Clone, Copy, Type, Serialize, Deserialize)]
#[sqlx(type_name = "chain_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ChainType {
    Integer,
    Float,
    Boolean,
}

#[derive(Debug, Clone, Copy, Type, Serialize, Deserialize)]
#[sqlx(type_name = "chain_aggregate", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ChainAggregate {
    Sum,
    Avg,
}

#[derive(Deserialize)]
pub struct CreateChainData {
    pub name: String,
    pub r#type: ChainType,
    pub aggregate: ChainAggregate,
    pub description: Option<String>,
    pub order: Option<i32>,
}

#[derive(Deserialize)]
pub struct UpdateChainRequest {
    pub name: Option<String>,
    pub r#type: Option<ChainType>,
    pub aggregate: Option<ChainAggregate>,
    pub description: Option<String>,
    pub order: Option<i32>,
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
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
) -> Result<Json<Vec<Chain>>, StatusCode> {
    let current_user = find_current_user(&pool, &cookie_jar).await?;

    let chains = sqlx::query_as::<_, Chain>(
        r#"
        SELECT *
        FROM chains
        WHERE user_id = $1
        ORDER BY id DESC
        "#,
    )
    .bind(current_user.id)
    .fetch_all(&pool)
    .await
    .map_err(|err| {
        dbg!(err);
        StatusCode::BAD_REQUEST
    })?;

    Ok(Json(chains))
}

pub async fn create(
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
    Json(data): Json<CreateChainData>,
) -> Result<(StatusCode, Json<Chain>), StatusCode> {
    let current_user = find_current_user(&pool, &cookie_jar).await?;

    let chain = sqlx::query_as::<_, Chain>(
        r#"
        INSERT INTO chains (
            user_id,
            name,
            type,
            aggregate,
            description,
            "order"
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(current_user.id)
    .bind(data.name)
    .bind(data.r#type)
    .bind(data.aggregate)
    .bind(data.description)
    .bind(data.order)
    .fetch_one(&pool)
    .await
    .map_err(|err| {
        dbg!(err);
        StatusCode::BAD_REQUEST
    })?;

    Ok((StatusCode::CREATED, Json(chain)))
}

pub async fn show(
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
    Path(chain_id): Path<i64>,
) -> Result<Json<Chain>, StatusCode> {
    let current_user = find_current_user(&pool, &cookie_jar).await?;
    let chain = find_chain(&pool, current_user.id, chain_id).await?;

    Ok(Json(chain))
}

async fn find_chain(pool: &PgPool, user_id: i64, chain_id: i64) -> Result<Chain, StatusCode> {
    let chain = sqlx::query_as::<_, Chain>(
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
        dbg!(err);
        StatusCode::NOT_FOUND
    })?;

    Ok(chain)
}

pub async fn update(
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
    Path(chain_id): Path<i64>,
    Json(data): Json<UpdateChainRequest>,
) -> Result<Json<Chain>, StatusCode> {
    let current_user = find_current_user(&pool, &cookie_jar).await?;

    let chain = sqlx::query_as::<_, Chain>(
        r#"
        UPDATE chains
        SET
            name = COALESCE($1, name),
            type = COALESCE($2, type),
            aggregate = COALESCE($3, aggregate),
            description = COALESCE($4, description),
            "order" = COALESCE($5, "order"),
            updated_at = NOW()
        WHERE id = $6 AND user_id = $7
        RETURNING *
        "#,
    )
    .bind(data.name)
    .bind(data.r#type)
    .bind(data.aggregate)
    .bind(data.description)
    .bind(data.order)
    .bind(chain_id)
    .bind(current_user.id)
    .fetch_one(&pool)
    .await
    .map_err(|err| {
        dbg!(err);
        StatusCode::BAD_REQUEST
    })?;

    Ok(Json(chain))
}

pub async fn delete(
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
    Path(chain_id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    let current_user = find_current_user(&pool, &cookie_jar).await?;

    sqlx::query(
        r#"
        DELETE FROM chains
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(chain_id)
    .bind(current_user.id)
    .execute(&pool)
    .await
    .map_err(|err| {
        dbg!(err);
        StatusCode::BAD_REQUEST
    })?;

    Ok(StatusCode::NO_CONTENT)
}

async fn find_current_user(pool: &PgPool, cookie_jar: &CookieJar) -> Result<User, StatusCode> {
    let jwt = cookie_jar
        .get("jwt")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .value();

    let token_data = decode_jwt(jwt.to_string()).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, email, password_hash
        FROM users
        WHERE email = $1
        "#,
    )
    .bind(token_data.claims.email)
    .fetch_one(pool)
    .await
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(user)
}

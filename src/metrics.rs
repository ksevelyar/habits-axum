use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;

use crate::chains::ChainType;
use crate::users::{CurrentUser, decode_jwt};

#[derive(Serialize)]
pub struct HistoryResponse {
    pub chains: Vec<ChainDTO>,
    pub sprints: Vec<SprintDTO>,
}

#[derive(Serialize)]
pub struct ChainDTO {
    pub name: String,
    pub id: i64,
    pub r#type: String,
    pub aggregate: String,
}

#[derive(Serialize)]
pub struct SprintDTO {
    pub total: HashMap<i64, f64>,
    pub week: HashMap<String, HashMap<i64, MetricDTO>>,
}

#[derive(Serialize, Clone)]
pub struct MetricDTO {
    pub id: i64,
    pub value: f64,
    pub date: String,
    pub chain: String,
    pub chain_id: i64,
}

#[derive(Deserialize)]
pub struct UpdateMetricData {
    pub date: NaiveDate,
    pub value: String,
    pub chain_id: i64,
}

#[derive(Deserialize)]
pub struct MetricsQuery {
    pub date: NaiveDate,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Metric {
    pub id: i64,
    pub chain_id: i64,
    pub date: NaiveDate,
    pub value_integer: Option<i64>,
    pub value_float: Option<f64>,
    pub value_bool: Option<bool>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct MetricByDate {
    pub id: Option<i64>,
    pub value: Option<f64>,
    pub updated_at: Option<chrono::DateTime<Utc>>,
    pub chain: String,
    pub chain_id: i64,
}

pub async fn upsert(
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
    Json(data): Json<UpdateMetricData>,
) -> Result<Json<Metric>, StatusCode> {
    let current_user = find_current_user(&pool, &cookie_jar).await?;

    let chain_type = sqlx::query_scalar!(
        r#"
        SELECT type as "type: ChainType"
        FROM chains
        WHERE id = $1
          AND user_id = $2
        "#,
        data.chain_id,
        current_user.id
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    let (value_integer, value_float, value_bool) = match chain_type {
        ChainType::Integer => {
            let v = data
                .value
                .parse::<i64>()
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            (Some(v), None, None)
        }
        ChainType::Float => {
            let v = data
                .value
                .parse::<f64>()
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            (None, Some(v), None)
        }
        ChainType::Boolean => {
            let v = match data.value.as_str() {
                "true" => true,
                "false" => false,
                _ => return Err(StatusCode::BAD_REQUEST),
            };
            (None, None, Some(v))
        }
    };

    let row = sqlx::query_as::<_, Metric>(
        r#"
        INSERT INTO metrics (
            chain_id,
            date,
            value_integer,
            value_float,
            value_bool
        )
        VALUES ($1, $2, $3, $4, $5)

        ON CONFLICT (chain_id, date)
        DO UPDATE SET
            value_integer = EXCLUDED.value_integer,
            value_float   = EXCLUDED.value_float,
            value_bool    = EXCLUDED.value_bool,
            updated_at    = NOW()

        RETURNING id, chain_id, date, value_integer, value_float, value_bool
        "#,
    )
    .bind(data.chain_id)
    .bind(data.date)
    .bind(value_integer)
    .bind(value_float)
    .bind(value_bool)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(row))
}

pub async fn delete(
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
    Path(metric_id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    let current_user = find_current_user(&pool, &cookie_jar).await?;

    sqlx::query(
        r#"
        DELETE FROM metrics
        WHERE id = $1
          AND chain_id IN (
              SELECT id
              FROM chains
              WHERE user_id = $2
          )
        "#,
    )
    .bind(metric_id)
    .bind(current_user.id)
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_by_date(
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
    Query(params): Query<MetricsQuery>,
) -> Result<Json<Vec<MetricByDate>>, StatusCode> {
    let current_user = find_current_user(&pool, &cookie_jar).await?;

    let rows = sqlx::query_as::<_, MetricByDate>(
        r#"
        SELECT
            m.id,

            COALESCE(
                m.value_float,
                m.value_integer::float,
                CASE WHEN m.value_bool THEN 1.0 ELSE 0.0 END
            ) AS value,

            m.updated_at,

            c.name AS chain,
            c.id AS chain_id

        FROM chains c
        LEFT JOIN metrics m
            ON m.chain_id = c.id
           AND m.date = $1

        WHERE c.active = TRUE
          AND c.user_id = $2

        ORDER BY c."order"
        "#,
    )
    .bind(params.date)
    .bind(current_user.id)
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(rows))
}

pub async fn history(
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
) -> Result<Json<HistoryResponse>, StatusCode> {
    let user = find_current_user(&pool, &cookie_jar).await?;

    let today = Utc::now().date_naive();
    let week_start = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let start = week_start - Duration::days(7);

    let chains = sqlx::query_as!(
        ChainDTO,
        r#"
        SELECT
            id,
            name,
            type::text AS "type!",
            aggregate::text AS "aggregate!"
        FROM chains
        WHERE user_id = $1
          AND active = true
        ORDER BY "order"
        "#,
        user.id
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::BAD_REQUEST)?;

    let metrics = sqlx::query_as!(
        MetricDTO,
        r#"
        SELECT
            m.id,

            COALESCE(
                m.value_float,
                m.value_integer::float8,
                CASE WHEN m.value_bool THEN 1.0 ELSE 0.0 END
            )::float8 AS "value!",

            m.date::text AS "date!",
            c.name AS "chain!",
            c.id AS "chain_id!"

        FROM metrics m
        JOIN chains c ON c.id = m.chain_id
        WHERE c.user_id = $1
          AND c.active = true
          AND m.date >= $2
        ORDER BY m.date ASC, c.id ASC
        "#,
        user.id,
        start
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut total: HashMap<i64, f64> = HashMap::new();
    let mut week: HashMap<String, HashMap<i64, MetricDTO>> = HashMap::new();

    for m in &metrics {
        *total.entry(m.chain_id).or_insert(0.0) += m.value;

        week.entry(m.date.clone())
            .or_default()
            .insert(m.chain_id, m.clone());
    }

    Ok(Json(HistoryResponse {
        chains,
        sprints: vec![SprintDTO { total, week }],
    }))
}

async fn find_current_user(
    pool: &PgPool,
    cookie_jar: &CookieJar,
) -> Result<CurrentUser, StatusCode> {
    let jwt = cookie_jar
        .get("jwt")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .value();

    let token_data = decode_jwt(jwt.to_string()).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user = sqlx::query_as::<_, CurrentUser>(
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

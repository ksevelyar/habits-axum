use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use crate::chains::ChainType;
use crate::error::AppError;
use crate::users::authenticate_user;

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
    pub week: BTreeMap<String, HashMap<i64, MetricDTO>>,
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
pub struct UpdateMetricPayload {
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
    pub value_integer: Option<i32>,
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
    State(state): State<Arc<crate::AppState>>,
    cookie_jar: CookieJar,
    Json(data): Json<UpdateMetricPayload>,
) -> Result<Json<Metric>, AppError> {
    let current_user = authenticate_user(&state.pool, &cookie_jar).await?;

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
    .fetch_one(&state.pool)
    .await
    .map_err(|_| AppError::NotFound("chain not found".into()))?;

    let (value_integer, value_float, value_bool) = match chain_type {
        ChainType::Integer => {
            let v = data
                .value
                .parse::<i64>()
                .map_err(|_| AppError::BadRequest("invalid integer".into()))?;
            (Some(v), None, None)
        }
        ChainType::Float => {
            let v = data
                .value
                .parse::<f64>()
                .map_err(|_| AppError::BadRequest("invalid float".into()))?;
            (None, Some(v), None)
        }
        ChainType::Boolean => match data.value.as_str() {
            "true" => (None, None, Some(true)),
            "false" => (None, None, Some(false)),
            _ => {
                return Err(AppError::BadRequest(
                    "invalid boolean, use true/false".into(),
                ));
            }
        },
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
    .fetch_one(&state.pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::BadRequest("database error".into())
    })?;

    Ok(Json(row))
}

pub async fn delete(
    State(state): State<Arc<crate::AppState>>,
    cookie_jar: CookieJar,
    Path(metric_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let current_user = authenticate_user(&state.pool, &cookie_jar).await?;

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
    .execute(&state.pool)
    .await
    .map_err(|_| AppError::BadRequest("database error".into()))?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_by_date(
    State(state): State<Arc<crate::AppState>>,
    cookie_jar: CookieJar,
    Query(params): Query<MetricsQuery>,
) -> Result<Json<Vec<MetricByDate>>, AppError> {
    let current_user = authenticate_user(&state.pool, &cookie_jar).await?;

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
    .fetch_all(&state.pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::BadRequest("database error".into())
    })?;

    Ok(Json(rows))
}

pub async fn history(
    State(state): State<Arc<crate::AppState>>,
    cookie_jar: CookieJar,
) -> Result<Json<HistoryResponse>, AppError> {
    let user = authenticate_user(&state.pool, &cookie_jar).await?;

    let today = Utc::now().date_naive();
    let week_start = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let prev_week_start = week_start - Duration::days(7);
    let week_start_str = week_start.to_string();

    let chains = sqlx::query_as!(
        ChainDTO,
        r#"
        SELECT id, name, type::text AS "type!", aggregate::text AS "aggregate!"
        FROM chains
        WHERE user_id = $1 AND active = true
        ORDER BY "order"
        "#,
        user.id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| AppError::BadRequest("database error".into()))?;

    let metrics = sqlx::query_as!(
        MetricDTO,
        r#"
        SELECT m.id,
               COALESCE(m.value_float, m.value_integer::float8,
                        CASE WHEN m.value_bool THEN 1.0 ELSE 0.0 END)::float8 AS "value!",
               m.date::text AS "date!",
               c.name AS "chain!", c.id AS "chain_id!"
        FROM metrics m
        JOIN chains c ON c.id = m.chain_id
        WHERE c.user_id = $1 AND c.active = true AND m.date >= $2
        ORDER BY m.date ASC, c.id ASC
        "#,
        user.id,
        prev_week_start
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| AppError::BadRequest("database error".into()))?;

    let chain_aggs: HashMap<i64, String> =
        chains.iter().map(|c| (c.id, c.aggregate.clone())).collect();

    #[derive(Default)]
    struct SprintAccum {
        sums: HashMap<i64, f64>,
        counts: HashMap<i64, usize>,
        week: BTreeMap<String, HashMap<i64, MetricDTO>>,
    }
    let mut acc = [SprintAccum::default(), SprintAccum::default()];

    for m in &metrics {
        let idx = if m.date >= week_start_str { 1 } else { 0 };
        let s = &mut acc[idx];
        *s.sums.entry(m.chain_id).or_insert(0.0) += m.value;
        *s.counts.entry(m.chain_id).or_insert(0) += 1;
        s.week
            .entry(m.date.clone())
            .or_default()
            .insert(m.chain_id, m.clone());
    }

    let sprints: Vec<SprintDTO> = acc
        .into_iter()
        .map(|a| {
            let mut total: HashMap<i64, f64> = chains.iter().map(|c| (c.id, 0.0)).collect();

            for (chain_id, sum) in a.sums {
                let count = a.counts.get(&chain_id).copied().unwrap_or(0);
                let agg = chain_aggs
                    .get(&chain_id)
                    .map(|s| s.as_str())
                    .unwrap_or("sum");

                let val = if agg == "avg" && count > 0 {
                    (sum / (count as f64) * 10.0).round() / 10.0
                } else {
                    sum
                };
                total.insert(chain_id, val);
            }

            SprintDTO {
                total,
                week: a.week,
            }
        })
        .collect();

    Ok(Json(HistoryResponse {
        chains,
        sprints: { sprints },
    }))
}

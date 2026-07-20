use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde::Serialize;
use sqlx::PgPool;
use std::collections::{BTreeMap, HashMap};

use crate::error::AppError;

#[derive(Serialize)]
pub struct HistoryResponse {
    pub chains: Vec<ChainInfo>,
    pub sprints: Vec<SprintInfo>,
}

#[derive(Serialize)]
pub struct ChainInfo {
    pub name: String,
    pub id: i64,
    pub r#type: String,
    pub aggregate: String,
}

#[derive(Serialize)]
pub struct SprintInfo {
    pub total: HashMap<i64, f64>,
    pub week: BTreeMap<String, HashMap<i64, MetricInfo>>,
}

#[derive(Serialize, Clone)]
pub struct MetricInfo {
    pub id: i64,
    pub value: f64,
    pub date: String,
    pub chain: String,
    pub chain_id: i64,
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

pub async fn upsert_metric(
    pool: &PgPool,
    chain_id: i64,
    date: NaiveDate,
    value_integer: Option<i64>,
    value_float: Option<f64>,
    value_bool: Option<bool>,
) -> Result<Metric, AppError> {
    sqlx::query_as::<_, Metric>(
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
    .bind(chain_id)
    .bind(date)
    .bind(value_integer)
    .bind(value_float)
    .bind(value_bool)
    .fetch_one(pool)
    .await
    .map_err(|err| {
        tracing::error!("{err}");
        AppError::BadRequest("database error".into())
    })
}

pub async fn delete_by_id(pool: &PgPool, metric_id: i64, user_id: i64) -> Result<(), AppError> {
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
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|_| AppError::BadRequest("database error".into()))?;

    Ok(())
}

pub async fn list_by_date(pool: &PgPool, user_id: i64, date: NaiveDate) -> Result<Vec<MetricByDate>, sqlx::Error> {
    sqlx::query_as::<_, MetricByDate>(
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
    .bind(date)
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn compute_history(pool: &PgPool, user_id: i64) -> Result<HistoryResponse, AppError> {
    let today = Utc::now().date_naive();
    let week_start = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let prev_week_start = week_start - Duration::days(7);
    let week_start_str = week_start.to_string();

    let chains = sqlx::query_as!(
        ChainInfo,
        r#"
        SELECT id, name, type::text AS "type!", aggregate::text AS "aggregate!"
        FROM chains
        WHERE user_id = $1 AND active = true
        ORDER BY "order"
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|_| AppError::BadRequest("database error".into()))?;

    let metrics = sqlx::query_as!(
        MetricInfo,
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
        user_id,
        prev_week_start
    )
    .fetch_all(pool)
    .await
    .map_err(|_| AppError::BadRequest("database error".into()))?;

    let chain_aggs: HashMap<i64, String> = chains.iter().map(|c| (c.id, c.aggregate.clone())).collect();

    #[derive(Default)]
    struct SprintAccum {
        sums: HashMap<i64, f64>,
        counts: HashMap<i64, usize>,
        week: BTreeMap<String, HashMap<i64, MetricInfo>>,
    }
    let mut acc = [SprintAccum::default(), SprintAccum::default()];

    for m in &metrics {
        let idx = if m.date >= week_start_str { 1 } else { 0 };
        let s = &mut acc[idx];
        *s.sums.entry(m.chain_id).or_insert(0.0) += m.value;
        *s.counts.entry(m.chain_id).or_insert(0) += 1;
        s.week.entry(m.date.clone()).or_default().insert(m.chain_id, m.clone());
    }

    let sprints: Vec<SprintInfo> = acc
        .into_iter()
        .map(|a| {
            let mut total: HashMap<i64, f64> = chains.iter().map(|c| (c.id, 0.0)).collect();

            for (chain_id, sum) in a.sums {
                let count = a.counts.get(&chain_id).copied().unwrap_or(0);
                let agg = chain_aggs.get(&chain_id).map(|s| s.as_str()).unwrap_or("sum");

                let val = if agg == "avg" && count > 0 {
                    (sum / (count as f64) * 10.0).round() / 10.0
                } else {
                    sum
                };
                total.insert(chain_id, val);
            }

            SprintInfo { total, week: a.week }
        })
        .collect();

    Ok(HistoryResponse {
        chains,
        sprints: { sprints },
    })
}

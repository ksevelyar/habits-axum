use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde::Serialize;
use sqlx::PgPool;
use std::collections::{BTreeMap, HashMap};
use tracing::Level;

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
    pub week: BTreeMap<NaiveDate, HashMap<i64, MetricInfo>>,
}

#[derive(Serialize, Clone)]
pub struct MetricInfo {
    pub id: i64,
    pub value: f64,
    pub date: NaiveDate,
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

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn upsert_metric(
    pool: &PgPool,
    chain_id: i64,
    date: NaiveDate,
    value_integer: Option<i64>,
    value_float: Option<f64>,
    value_bool: Option<bool>,
) -> Result<Metric, AppError> {
    Ok(sqlx::query_as::<_, Metric>(
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
    .await?)
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn delete_by_id(pool: &PgPool, metric_id: i64, user_id: i64) -> Result<(), AppError> {
    let deletion = sqlx::query(
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
    .await?;

    if deletion.rows_affected() == 0 {
        return Err(AppError::NotFound("metric not found".into()));
    }
    Ok(())
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn list_by_date(pool: &PgPool, user_id: i64, date: NaiveDate) -> Result<Vec<MetricByDate>, AppError> {
    Ok(sqlx::query_as::<_, MetricByDate>(
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
    .await?)
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn compute_history(pool: &PgPool, user_id: i64) -> Result<HistoryResponse, AppError> {
    let today = Utc::now().date_naive();
    let week_start = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let prev_week_start = week_start - Duration::days(7);

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
    .await?;

    let metrics = sqlx::query_as!(
        MetricInfo,
        r#"
        SELECT m.id,
               COALESCE(m.value_float, m.value_integer::float8,
                        CASE WHEN m.value_bool THEN 1.0 ELSE 0.0 END)::float8 AS "value!",
               m.date,
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
    .await?;

    let chain_aggregates: HashMap<i64, String> =
        chains.iter().map(|chain| (chain.id, chain.aggregate.clone())).collect();

    #[derive(Default)]
    struct SprintAccum {
        sums: HashMap<i64, f64>,
        counts: HashMap<i64, usize>,
        week: BTreeMap<NaiveDate, HashMap<i64, MetricInfo>>,
    }
    let mut accumulators = [SprintAccum::default(), SprintAccum::default()];

    for metric in &metrics {
        let sprint_index = usize::from(metric.date >= week_start);
        let accumulator = &mut accumulators[sprint_index];
        *accumulator.sums.entry(metric.chain_id).or_insert(0.0) += metric.value;
        *accumulator.counts.entry(metric.chain_id).or_insert(0) += 1;
        accumulator
            .week
            .entry(metric.date)
            .or_default()
            .insert(metric.chain_id, metric.clone());
    }

    let sprints: Vec<SprintInfo> = accumulators
        .into_iter()
        .map(|accumulator| {
            let mut total: HashMap<i64, f64> = chains.iter().map(|chain| (chain.id, 0.0)).collect();

            for (chain_id, sum) in accumulator.sums {
                let count = accumulator.counts.get(&chain_id).copied().unwrap_or(0);
                let aggregate = chain_aggregates.get(&chain_id).map(String::as_str).unwrap_or("sum");

                let aggregated = if aggregate == "avg" && count > 0 {
                    (sum / (count as f64) * 10.0).round() / 10.0
                } else {
                    sum
                };
                total.insert(chain_id, aggregated);
            }

            SprintInfo {
                total,
                week: accumulator.week,
            }
        })
        .collect();

    Ok(HistoryResponse { chains, sprints })
}

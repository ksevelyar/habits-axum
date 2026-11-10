use jiff::civil::Date;
use jiff::{Span, Timestamp};
use jiff_sqlx::ToSqlx;
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

#[derive(Serialize, sqlx::FromRow)]
pub struct ChainInfo {
    pub name: String,
    pub id: i64,
    pub r#type: String,
    pub aggregate: String,
}

#[derive(Serialize)]
pub struct SprintInfo {
    pub total: HashMap<i64, f64>,
    pub week: BTreeMap<Date, HashMap<i64, MetricInfo>>,
}

#[derive(Serialize, Clone, sqlx::FromRow)]
pub struct MetricInfo {
    pub id: i64,
    pub value: f64,
    #[sqlx(try_from = "jiff_sqlx::Date")]
    pub date: Date,
    pub chain: String,
    pub chain_id: i64,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Metric {
    pub id: i64,
    pub chain_id: i64,
    #[sqlx(try_from = "jiff_sqlx::Date")]
    pub date: Date,
    pub value_integer: Option<i32>,
    pub value_float: Option<f64>,
    pub value_bool: Option<bool>,
}

#[derive(Serialize)]
pub struct MetricByDate {
    pub id: Option<i64>,
    pub value: Option<f64>,
    pub updated_at: Option<Timestamp>,
    pub chain: String,
    pub chain_id: i64,
}

#[derive(sqlx::FromRow)]
struct MetricByDateRow {
    id: Option<i64>,
    value: Option<f64>,
    updated_at: Option<jiff_sqlx::Timestamp>,
    chain: String,
    chain_id: i64,
}

impl From<MetricByDateRow> for MetricByDate {
    fn from(row: MetricByDateRow) -> Self {
        Self {
            id: row.id,
            value: row.value,
            updated_at: row.updated_at.map(Into::into),
            chain: row.chain,
            chain_id: row.chain_id,
        }
    }
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn upsert_metric(
    pool: &PgPool,
    chain_id: i64,
    date: Date,
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
    .bind(date.to_sqlx())
    .bind(value_integer)
    .bind(value_float)
    .bind(value_bool)
    .fetch_one(pool)
    .await?)
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn delete_metric(pool: &PgPool, chain_id: i64, date: Date) -> Result<(), AppError> {
    sqlx::query(
        r#"
        DELETE FROM metrics
        WHERE chain_id = $1
          AND date = $2
        "#,
    )
    .bind(chain_id)
    .bind(date.to_sqlx())
    .execute(pool)
    .await?;

    Ok(())
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn list_by_date(pool: &PgPool, user_id: i64, date: Date) -> Result<Vec<MetricByDate>, AppError> {
    let metric_rows = sqlx::query_as::<_, MetricByDateRow>(
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
    .bind(date.to_sqlx())
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(metric_rows.into_iter().map(Into::into).collect())
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn compute_history(pool: &PgPool, user_id: i64) -> Result<HistoryResponse, AppError> {
    let today = Timestamp::now().to_zoned(jiff::tz::TimeZone::UTC).date();
    let days_since_monday = i64::from(today.weekday().to_monday_zero_offset());
    let week_start = today
        .checked_sub(Span::new().days(days_since_monday))
        .expect("the current date has a previous Monday");
    let previous_week_start = week_start
        .checked_sub(Span::new().days(7))
        .expect("the current date has a previous week");

    let chains = sqlx::query_as::<_, ChainInfo>(
        r#"
        SELECT id, name, type::text AS type, aggregate::text AS aggregate
        FROM chains
        WHERE user_id = $1 AND active = true
        ORDER BY "order"
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let metrics = sqlx::query_as::<_, MetricInfo>(
        r#"
        SELECT m.id,
               COALESCE(m.value_float, m.value_integer::float8,
                        CASE WHEN m.value_bool THEN 1.0 ELSE 0.0 END)::float8 AS value,
               m.date,
               c.name AS chain, c.id AS chain_id
        FROM metrics m
        JOIN chains c ON c.id = m.chain_id
        WHERE c.user_id = $1 AND c.active = true AND m.date >= $2
        ORDER BY m.date ASC, c.id ASC
        "#,
    )
    .bind(user_id)
    .bind(previous_week_start.to_sqlx())
    .fetch_all(pool)
    .await?;

    let chain_aggregates: HashMap<i64, String> =
        chains.iter().map(|chain| (chain.id, chain.aggregate.clone())).collect();

    #[derive(Default)]
    struct SprintAccumulator {
        sums: HashMap<i64, f64>,
        counts: HashMap<i64, usize>,
        week: BTreeMap<Date, HashMap<i64, MetricInfo>>,
    }
    let mut accumulators = [SprintAccumulator::default(), SprintAccumulator::default()];

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

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::types::Json;
use tracing::Level;

use crate::chains::ChainType;
use crate::error::AppError;

#[derive(Deserialize, Serialize)]
pub struct ChartsResponse {
    pub chains: Vec<ChartHistoryChain>,
}

#[derive(Deserialize, Serialize)]
pub struct ChartHistoryChain {
    pub id: i64,
    pub name: String,
    pub r#type: ChainType,
    pub points: Vec<ChartHistoryPoint>,
}

#[derive(Deserialize, Serialize)]
pub struct ChartHistoryPoint {
    pub date: NaiveDate,
    pub value: f64,
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn list_history(
    pool: &PgPool,
    user_id: i64,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<ChartsResponse, AppError> {
    Ok(sqlx::query_scalar::<_, Json<ChartsResponse>>(
        r#"
        SELECT jsonb_build_object(
            'chains', COALESCE(
                jsonb_agg(
                    jsonb_build_object(
                        'id', chains.id,
                        'name', chains.name,
                        'type', chains.type,
                        'points', COALESCE(chart_points.points, '[]'::jsonb)
                    )
                    ORDER BY chains."order" NULLS LAST, chains.id
                ),
                '[]'::jsonb
            )
        )
        FROM chains
        LEFT JOIN LATERAL (
            SELECT jsonb_agg(
                jsonb_build_object(
                    'date', metrics.date,
                    'value', COALESCE(
                        metrics.value_float,
                        metrics.value_integer::float8,
                        CASE WHEN metrics.value_bool THEN 1.0 ELSE 0.0 END
                    )
                )
                ORDER BY metrics.date
            ) AS points
            FROM metrics
            WHERE metrics.chain_id = chains.id
              AND metrics.date >= $2
              AND metrics.date <= $3
        ) AS chart_points ON TRUE
        WHERE chains.user_id = $1
          AND chains.active = TRUE
          AND chains.show_in_charts = TRUE
        "#,
    )
    .bind(user_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_one(pool)
    .await?
    .0)
}

use axum::{
    extract::{Json, Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use axum_extra::extract::cookie::CookieJar;
use chrono::NaiveDate;
use serde::Deserialize;
use std::sync::Arc;

use crate::AppState;
use crate::authentication::{authenticate_cookie, authenticate_token, extract_token};
use crate::chains::ChainType;
use crate::error::AppError;
use crate::metrics::{HistoryResponse, Metric, MetricByDate};

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

pub async fn upsert(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    headers: HeaderMap,
    Json(data): Json<UpdateMetricPayload>,
) -> Result<Json<Metric>, AppError> {
    let token = extract_token(&cookie_jar, &headers).ok_or(AppError::Unauthorized)?;
    let user = authenticate_token(&state.pool, token).await?;

    let chain_type = sqlx::query_scalar!(
        r#"
        SELECT type as "type: ChainType"
        FROM chains
        WHERE id = $1
          AND user_id = $2
        "#,
        data.chain_id,
        user.id
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
                return Err(AppError::BadRequest("invalid boolean, use true/false".into()));
            }
        },
        ChainType::Time => {
            let v = data
                .value
                .parse::<i64>()
                .map_err(|_| AppError::BadRequest("invalid time in minutes".into()))?;
            (Some(v), None, None)
        }
    };

    crate::metrics::upsert_metric(
        &state.pool,
        data.chain_id,
        data.date,
        value_integer,
        value_float,
        value_bool,
    )
    .await
    .map(Json)
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(metric_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::metrics::delete_by_id(&state.pool, metric_id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_by_date(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Query(params): Query<MetricsQuery>,
) -> Result<Json<Vec<MetricByDate>>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::metrics::list_by_date(&state.pool, user.id, params.date)
        .await
        .map_err(|err| {
            tracing::error!("{err}");
            AppError::BadRequest("database error".into())
        })
        .map(Json)
}

pub async fn history(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
) -> Result<Json<HistoryResponse>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::metrics::compute_history(&state.pool, user.id).await.map(Json)
}

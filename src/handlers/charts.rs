use axum::Json;
use axum::extract::{Query, State};
use axum_extra::extract::cookie::CookieJar;
use chrono::NaiveDate;
use serde::Deserialize;
use std::sync::Arc;

use crate::AppState;
use crate::authentication::authenticate_cookie;
use crate::charts::ChartsResponse;
use crate::error::AppError;

#[derive(Deserialize)]
pub struct ChartHistoryQuery {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

pub async fn show_history(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Query(query): Query<ChartHistoryQuery>,
) -> Result<Json<ChartsResponse>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;

    if query.start_date > query.end_date {
        return Err(AppError::BadRequest("start_date must be on or before end_date".into()));
    }

    crate::charts::list_history(&state.pool, user.id, query.start_date, query.end_date)
        .await
        .map(Json)
}

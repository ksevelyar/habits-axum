use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::Arc;

use crate::AppState;
use crate::authentication::authenticate_cookie;
use crate::error::{AppError, FieldError};
use crate::ledger::{CreateLedgerCategory, LedgerTransaction};

#[derive(Deserialize)]
pub struct CategoryNamePayload {
    pub name: String,
}

#[derive(Deserialize)]
pub struct TransactionPayload {
    pub category_id: i64,
    pub amount_kopecks: i64,
    pub occurred_at: String,
}

#[derive(Deserialize)]
pub struct ListTransactionsQuery {
    pub start_at: String,
    pub end_at: String,
}

fn deserialize_payload<T: DeserializeOwned>(value: Value) -> Result<T, AppError> {
    serde_path_to_error::deserialize(value).map_err(|error| {
        let error_message = error.inner().to_string();
        let field_path = error.path().to_string();
        let field = error_message
            .strip_prefix("missing field `")
            .and_then(|missing_field| missing_field.strip_suffix('`'))
            .map(str::to_owned)
            .unwrap_or(field_path);

        AppError::Validation(vec![FieldError {
            field,
            message: error_message,
        }])
    })
}

pub async fn list_categories(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
) -> Result<Json<Vec<crate::ledger::LedgerCategory>>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::ledger::list_categories(&state.pool, user.id).await.map(Json)
}

pub async fn create_category(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Json(value): Json<Value>,
) -> Result<(StatusCode, Json<crate::ledger::LedgerCategory>), AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let payload: CategoryNamePayload = deserialize_payload(value)?;
    crate::ledger::create_category(
        &state.pool,
        CreateLedgerCategory {
            user_id: user.id,
            name: payload.name,
        },
    )
    .await
    .map(|category| (StatusCode::CREATED, Json(category)))
}

pub async fn update_category(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(category_id): Path<i64>,
    Json(value): Json<Value>,
) -> Result<Json<crate::ledger::LedgerCategory>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let payload: CategoryNamePayload = deserialize_payload(value)?;
    crate::ledger::update_category(&state.pool, user.id, category_id, payload.name)
        .await
        .map(Json)
}

pub async fn delete_category(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(category_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::ledger::delete_category(&state.pool, user.id, category_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_transactions(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Query(query): Query<ListTransactionsQuery>,
) -> Result<Json<crate::ledger::LedgerHistory>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let start_at = crate::ledger::validate_utc_timestamp(&query.start_at, "start_at")?;
    let end_at = crate::ledger::validate_utc_timestamp(&query.end_at, "end_at")?;

    if start_at >= end_at {
        return Err(AppError::BadRequest("start_at must be before end_at".into()));
    }

    crate::ledger::list_transactions(&state.pool, user.id, start_at, end_at)
        .await
        .map(Json)
}

pub async fn create_transaction(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Json(value): Json<Value>,
) -> Result<(StatusCode, Json<LedgerTransaction>), AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let payload: TransactionPayload = deserialize_payload(value)?;
    let occurred_at = crate::ledger::validate_utc_timestamp(&payload.occurred_at, "occurred_at")?;
    crate::ledger::create_transaction(
        &state.pool,
        user.id,
        payload.category_id,
        payload.amount_kopecks,
        occurred_at,
    )
    .await
    .map(|transaction| (StatusCode::CREATED, Json(transaction)))
}

pub async fn update_transaction(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(transaction_id): Path<i64>,
    Json(value): Json<Value>,
) -> Result<Json<LedgerTransaction>, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    let payload: TransactionPayload = deserialize_payload(value)?;
    let occurred_at = crate::ledger::validate_utc_timestamp(&payload.occurred_at, "occurred_at")?;
    crate::ledger::update_transaction(
        &state.pool,
        user.id,
        transaction_id,
        payload.category_id,
        payload.amount_kopecks,
        occurred_at,
    )
    .await
    .map(Json)
}

pub async fn delete_transaction(
    State(state): State<Arc<AppState>>,
    cookie_jar: CookieJar,
    Path(transaction_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let user = authenticate_cookie(&state.pool, &cookie_jar).await?;
    crate::ledger::delete_transaction(&state.pool, user.id, transaction_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

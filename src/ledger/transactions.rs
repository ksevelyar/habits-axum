use jiff::Timestamp;
use jiff_sqlx::ToSqlx;
use serde::Serialize;
use sqlx::PgPool;
use tracing::Level;

use crate::error::AppError;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LedgerHistoryTransaction {
    pub id: i64,
    pub category_id: i64,
    pub category_name: String,
    pub amount_kopecks: i64,
    #[sqlx(try_from = "jiff_sqlx::Timestamp")]
    pub occurred_at: Timestamp,
    #[sqlx(try_from = "jiff_sqlx::Timestamp")]
    pub inserted_at: Timestamp,
    #[sqlx(try_from = "jiff_sqlx::Timestamp")]
    pub updated_at: Timestamp,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LedgerTransaction {
    pub id: i64,
    pub category_id: i64,
    pub amount_kopecks: i64,
    #[sqlx(try_from = "jiff_sqlx::Timestamp")]
    pub occurred_at: Timestamp,
    #[sqlx(try_from = "jiff_sqlx::Timestamp")]
    pub inserted_at: Timestamp,
    #[sqlx(try_from = "jiff_sqlx::Timestamp")]
    pub updated_at: Timestamp,
}

#[derive(Debug, Serialize)]
pub struct LedgerHistory {
    pub start_at: Timestamp,
    pub end_at: Timestamp,
    pub balance_kopecks: i64,
    pub transactions: Vec<LedgerHistoryTransaction>,
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn create_transaction(
    pool: &PgPool,
    user_id: i64,
    category_id: i64,
    amount_kopecks: i64,
    occurred_at: Timestamp,
) -> Result<LedgerTransaction, AppError> {
    validate_amount(amount_kopecks)?;

    sqlx::query_as::<_, LedgerTransaction>(
        r#"
        INSERT INTO ledger_transactions (
            category_id,
            amount_kopecks,
            occurred_at
        )
        SELECT id, $3, $4
        FROM ledger_categories
        WHERE id = $1
          AND user_id = $2
        RETURNING
            id,
            category_id,
            amount_kopecks,
            occurred_at,
            inserted_at,
            updated_at
        "#,
    )
    .bind(category_id)
    .bind(user_id)
    .bind(amount_kopecks)
    .bind(occurred_at.to_sqlx())
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("category not found".into()))
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn update_transaction(
    pool: &PgPool,
    user_id: i64,
    transaction_id: i64,
    category_id: i64,
    amount_kopecks: i64,
    occurred_at: Timestamp,
) -> Result<LedgerTransaction, AppError> {
    validate_amount(amount_kopecks)?;

    sqlx::query_as::<_, LedgerTransaction>(
        r#"
        UPDATE ledger_transactions
        SET
            category_id = $1,
            amount_kopecks = $2,
            occurred_at = $3,
            updated_at = NOW()
        WHERE id = $4
          AND EXISTS (
              SELECT 1
              FROM ledger_categories
              WHERE id = ledger_transactions.category_id
                AND user_id = $5
          )
          AND EXISTS (
              SELECT 1
              FROM ledger_categories
              WHERE id = $1
                AND user_id = $5
          )
        RETURNING
            id,
            category_id,
            amount_kopecks,
            occurred_at,
            inserted_at,
            updated_at
        "#,
    )
    .bind(category_id)
    .bind(amount_kopecks)
    .bind(occurred_at.to_sqlx())
    .bind(transaction_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("transaction not found".into()))
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn delete_transaction(pool: &PgPool, user_id: i64, transaction_id: i64) -> Result<(), AppError> {
    let deletion = sqlx::query(
        r#"
        DELETE FROM ledger_transactions AS transaction
        USING ledger_categories AS category
        WHERE transaction.id = $1
          AND transaction.category_id = category.id
          AND category.user_id = $2
        "#,
    )
    .bind(transaction_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    if deletion.rows_affected() == 0 {
        return Err(AppError::NotFound("transaction not found".into()));
    }
    Ok(())
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn list_transactions(
    pool: &PgPool,
    user_id: i64,
    start_at: Timestamp,
    end_at: Timestamp,
) -> Result<LedgerHistory, AppError> {
    let transactions = sqlx::query_as::<_, LedgerHistoryTransaction>(
        r#"
        SELECT
            transaction.id,
            transaction.category_id,
            category.name AS category_name,
            transaction.amount_kopecks,
            transaction.occurred_at,
            transaction.inserted_at,
            transaction.updated_at
        FROM ledger_transactions AS transaction
        JOIN ledger_categories AS category ON category.id = transaction.category_id
        WHERE category.user_id = $1
          AND transaction.occurred_at >= $2::timestamptz
          AND transaction.occurred_at < $3::timestamptz
        ORDER BY transaction.occurred_at DESC, transaction.id DESC
        "#,
    )
    .bind(user_id)
    .bind(start_at.to_sqlx())
    .bind(end_at.to_sqlx())
    .fetch_all(pool)
    .await?;

    let balance_kopecks: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(transaction.amount_kopecks), 0)::bigint
        FROM ledger_transactions AS transaction
        JOIN ledger_categories AS category ON category.id = transaction.category_id
        WHERE category.user_id = $1
          AND transaction.occurred_at < $2::timestamptz
        "#,
    )
    .bind(user_id)
    .bind(end_at.to_sqlx())
    .fetch_one(pool)
    .await?;

    Ok(LedgerHistory {
        start_at,
        end_at,
        balance_kopecks,
        transactions,
    })
}

pub fn validate_utc_timestamp(value: &str, field_name: &str) -> Result<Timestamp, AppError> {
    if !value.ends_with('Z') {
        return Err(AppError::BadRequest(format!(
            "{field_name} must be a UTC RFC 3339 timestamp ending in Z"
        )));
    }

    value
        .parse::<Timestamp>()
        .map_err(|_| AppError::BadRequest(format!("{field_name} must be a valid RFC 3339 timestamp")))
}

fn validate_amount(amount_kopecks: i64) -> Result<(), AppError> {
    if amount_kopecks == 0 {
        return Err(AppError::BadRequest("amount_kopecks cannot be zero".into()));
    }
    Ok(())
}

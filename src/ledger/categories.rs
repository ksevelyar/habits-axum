use jiff::Timestamp;
use serde::Serialize;
use sqlx::PgPool;
use tracing::Level;

use crate::error::AppError;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LedgerCategory {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    #[sqlx(try_from = "jiff_sqlx::Timestamp")]
    pub inserted_at: Timestamp,
    #[sqlx(try_from = "jiff_sqlx::Timestamp")]
    pub updated_at: Timestamp,
}

#[derive(Debug)]
pub struct CreateLedgerCategory {
    pub user_id: i64,
    pub name: String,
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn list_categories(pool: &PgPool, user_id: i64) -> Result<Vec<LedgerCategory>, AppError> {
    Ok(sqlx::query_as::<_, LedgerCategory>(
        r#"
        SELECT id, user_id, name, inserted_at, updated_at
        FROM ledger_categories
        WHERE user_id = $1
        ORDER BY id
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn create_category(pool: &PgPool, payload: CreateLedgerCategory) -> Result<LedgerCategory, AppError> {
    let category_name = payload.name.trim();
    if category_name.is_empty() {
        return Err(AppError::BadRequest("category name cannot be empty".into()));
    }

    sqlx::query_as::<_, LedgerCategory>(
        r#"
        INSERT INTO ledger_categories (user_id, name)
        VALUES ($1, $2)
        RETURNING id, user_id, name, inserted_at, updated_at
        "#,
    )
    .bind(payload.user_id)
    .bind(category_name)
    .fetch_one(pool)
    .await
    .map_err(|error| match error {
        sqlx::Error::Database(database_error) if database_error.is_unique_violation() => {
            AppError::BadRequest("category name already exists".into())
        }
        error => error.into(),
    })
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn update_category(
    pool: &PgPool,
    user_id: i64,
    category_id: i64,
    name: String,
) -> Result<LedgerCategory, AppError> {
    let category_name = name.trim();
    if category_name.is_empty() {
        return Err(AppError::BadRequest("category name cannot be empty".into()));
    }

    let updated_category = sqlx::query_as::<_, LedgerCategory>(
        r#"
        UPDATE ledger_categories
        SET name = $1, updated_at = NOW()
        WHERE id = $2 AND user_id = $3
        RETURNING id, user_id, name, inserted_at, updated_at
        "#,
    )
    .bind(category_name)
    .bind(category_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| match error {
        sqlx::Error::Database(database_error) if database_error.is_unique_violation() => {
            AppError::BadRequest("category name already exists".into())
        }
        error => error.into(),
    })?;

    updated_category.ok_or_else(|| AppError::NotFound("category not found".into()))
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn delete_category(pool: &PgPool, user_id: i64, category_id: i64) -> Result<(), AppError> {
    let deletion = sqlx::query("DELETE FROM ledger_categories WHERE id = $1 AND user_id = $2")
        .bind(category_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|error| match error {
            sqlx::Error::Database(database_error) if database_error.is_foreign_key_violation() => {
                AppError::BadRequest("category has transactions".into())
            }
            error => error.into(),
        })?;

    if deletion.rows_affected() == 0 {
        return Err(AppError::NotFound("category not found".into()));
    }
    Ok(())
}

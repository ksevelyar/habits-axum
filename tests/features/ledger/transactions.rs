use axum::http::StatusCode;
use axum::{body::Body, http::Request};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::super::support::{
    build_application, create_authenticated_user, create_category, create_transaction, delete_with_cookie,
    get_with_cookie, patch_json, post_json, read_json_body, send,
};

fn build_history_request(cookie: &str, start_at: &str, end_at: &str) -> Request<Body> {
    get_with_cookie(
        &format!("/ledger/transactions?start_at={start_at}&end_at={end_at}"),
        cookie,
    )
}

#[sqlx::test]
async fn user_records_work_income(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "ledger-income@example.com", "Europe/Moscow").await;
    let application = build_application(&pool);
    let work_category = create_category(&application, &cookie, "Work").await;
    let work_category_id = work_category["id"].as_i64().unwrap();

    let transaction = create_transaction(&application, &cookie, work_category_id, 150000, "2026-03-09T12:00:00Z").await;

    assert!(transaction["id"].as_i64().is_some());
    assert_eq!(transaction["category_id"], work_category_id);
    assert_eq!(transaction["amount_kopecks"], 150000);
    assert_eq!(transaction["occurred_at"], "2026-03-09T12:00:00Z");
    assert!(transaction["inserted_at"].is_string());
    assert!(transaction["updated_at"].is_string());
}

#[sqlx::test]
async fn user_records_home_spending(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "ledger-expense@example.com", "Europe/Moscow").await;
    let application = build_application(&pool);
    let home_category = create_category(&application, &cookie, "Home").await;
    let home_category_id = home_category["id"].as_i64().unwrap();

    let transaction = create_transaction(&application, &cookie, home_category_id, -15990, "2026-03-09T12:00:00Z").await;

    assert!(transaction["id"].as_i64().is_some());
    assert_eq!(transaction["category_id"], home_category_id);
    assert_eq!(transaction["amount_kopecks"], -15990);
    assert_eq!(transaction["occurred_at"], "2026-03-09T12:00:00Z");
    assert!(transaction["inserted_at"].is_string());
    assert!(transaction["updated_at"].is_string());
}

#[sqlx::test]
async fn user_must_provide_category(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "ledger-no-category@example.com", "UTC").await;
    let application = build_application(&pool);

    let response = send(
        &application,
        post_json(
            "/ledger/transactions",
            json!({"amount_kopecks": -100, "occurred_at": "2026-03-09T12:00:00Z"}),
            &cookie,
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = read_json_body(response).await;
    assert_eq!(body["error"], "validation failed");
    assert_eq!(body["fields"][0]["field"], "category_id");
    assert_eq!(body["fields"][0]["message"], "missing field `category_id`");
}

#[sqlx::test]
async fn user_must_provide_occurrence_timestamp(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "ledger-no-time@example.com", "UTC").await;
    let application = build_application(&pool);
    let category = create_category(&application, &cookie, "Food").await;

    let response = send(
        &application,
        post_json(
            "/ledger/transactions",
            json!({"category_id": category["id"], "amount_kopecks": -100}),
            &cookie,
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = read_json_body(response).await;
    assert_eq!(body["fields"][0]["field"], "occurred_at");
    assert_eq!(body["fields"][0]["message"], "missing field `occurred_at`");
}

#[sqlx::test]
async fn user_cannot_submit_zero_amount(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "ledger-zero-amount@example.com", "UTC").await;
    let application = build_application(&pool);
    let category = create_category(&application, &cookie, "Food").await;

    let response = send(
        &application,
        post_json(
            "/ledger/transactions",
            json!({
                "category_id": category["id"],
                "amount_kopecks": 0,
                "occurred_at": "2026-03-09T12:00:00Z"
            }),
            &cookie,
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_json_body(response).await,
        json!({"error": "amount_kopecks cannot be zero"})
    );
}

#[sqlx::test]
async fn user_cannot_submit_timestamp_that_is_not_utc(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "ledger-non-utc@example.com", "UTC").await;
    let application = build_application(&pool);
    let category = create_category(&application, &cookie, "Food").await;

    let response = send(
        &application,
        post_json(
            "/ledger/transactions",
            json!({
                "category_id": category["id"],
                "amount_kopecks": -100,
                "occurred_at": "2026-03-09T15:00:00+03:00"
            }),
            &cookie,
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_json_body(response).await,
        json!({"error": "occurred_at must be a UTC RFC 3339 timestamp ending in Z"})
    );
}

#[sqlx::test]
async fn user_cannot_create_transaction_in_another_users_category(pool: PgPool) {
    let (_, owner_cookie) = create_authenticated_user(&pool, "ledger-foreign-category-owner@example.com", "UTC").await;
    let (_, another_cookie) = create_authenticated_user(&pool, "ledger-foreign-category-user@example.com", "UTC").await;
    let application = build_application(&pool);
    let foreign_category = create_category(&application, &owner_cookie, "Work").await;

    let response = send(
        &application,
        post_json(
            "/ledger/transactions",
            json!({
                "category_id": foreign_category["id"],
                "amount_kopecks": 150000,
                "occurred_at": "2026-03-09T12:00:00Z"
            }),
            &another_cookie,
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(read_json_body(response).await, json!({"error": "category not found"}));
}

#[sqlx::test]
async fn user_updates_transaction(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "ledger-update@example.com", "UTC").await;
    let application = build_application(&pool);
    let food_category = create_category(&application, &cookie, "Food").await;
    let snack_category = create_category(&application, &cookie, "Snack").await;
    let transaction = create_transaction(
        &application,
        &cookie,
        food_category["id"].as_i64().unwrap(),
        -900,
        "2026-03-09T12:00:00Z",
    )
    .await;

    let response = send(
        &application,
        patch_json(
            &format!("/ledger/transactions/{}", transaction["id"].as_i64().unwrap()),
            json!({
                "category_id": snack_category["id"],
                "amount_kopecks": -1200,
                "occurred_at": "2026-03-10T14:30:00Z"
            }),
            &cookie,
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated_transaction = read_json_body(response).await;
    assert_eq!(updated_transaction["id"], transaction["id"]);
    assert_eq!(updated_transaction["category_id"], snack_category["id"]);
    assert_eq!(updated_transaction["amount_kopecks"], -1200);
    assert_eq!(updated_transaction["occurred_at"], "2026-03-10T14:30:00Z");
    assert!(updated_transaction["inserted_at"].is_string());
    assert!(updated_transaction["updated_at"].is_string());
}

#[sqlx::test]
async fn user_must_provide_every_field_when_updating_transaction(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "ledger-update-missing-field@example.com", "UTC").await;
    let application = build_application(&pool);
    let category = create_category(&application, &cookie, "Food").await;
    let transaction = create_transaction(
        &application,
        &cookie,
        category["id"].as_i64().unwrap(),
        -900,
        "2026-03-09T12:00:00Z",
    )
    .await;

    let response = send(
        &application,
        patch_json(
            &format!("/ledger/transactions/{}", transaction["id"].as_i64().unwrap()),
            json!({"category_id": category["id"], "amount_kopecks": -1200}),
            &cookie,
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = read_json_body(response).await;
    assert_eq!(body["fields"][0]["field"], "occurred_at");
    assert_eq!(body["fields"][0]["message"], "missing field `occurred_at`");
}

#[sqlx::test]
async fn user_cannot_update_transaction_to_another_users_category(pool: PgPool) {
    let (_, owner_cookie) = create_authenticated_user(&pool, "ledger-update-category-owner@example.com", "UTC").await;
    let (_, another_cookie) = create_authenticated_user(&pool, "ledger-update-category-user@example.com", "UTC").await;
    let application = build_application(&pool);
    let owner_category = create_category(&application, &owner_cookie, "Food").await;
    let foreign_category = create_category(&application, &another_cookie, "Foreign").await;
    let transaction = create_transaction(
        &application,
        &owner_cookie,
        owner_category["id"].as_i64().unwrap(),
        -900,
        "2026-03-09T12:00:00Z",
    )
    .await;

    let response = send(
        &application,
        patch_json(
            &format!("/ledger/transactions/{}", transaction["id"].as_i64().unwrap()),
            json!({
                "category_id": foreign_category["id"],
                "amount_kopecks": -1200,
                "occurred_at": "2026-03-10T14:30:00Z"
            }),
            &owner_cookie,
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        read_json_body(response).await,
        json!({"error": "transaction not found"})
    );
}

#[sqlx::test]
async fn user_cannot_update_transaction_with_zero_amount(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "ledger-update-zero-amount@example.com", "UTC").await;
    let application = build_application(&pool);
    let category = create_category(&application, &cookie, "Food").await;
    let transaction = create_transaction(
        &application,
        &cookie,
        category["id"].as_i64().unwrap(),
        -900,
        "2026-03-09T12:00:00Z",
    )
    .await;

    let response = send(
        &application,
        patch_json(
            &format!("/ledger/transactions/{}", transaction["id"].as_i64().unwrap()),
            json!({
                "category_id": category["id"],
                "amount_kopecks": 0,
                "occurred_at": "2026-03-10T14:30:00Z"
            }),
            &cookie,
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_json_body(response).await,
        json!({"error": "amount_kopecks cannot be zero"})
    );
}

#[sqlx::test]
async fn user_cannot_update_transaction_with_non_utc_timestamp(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "ledger-update-non-utc@example.com", "UTC").await;
    let application = build_application(&pool);
    let category = create_category(&application, &cookie, "Food").await;
    let transaction = create_transaction(
        &application,
        &cookie,
        category["id"].as_i64().unwrap(),
        -900,
        "2026-03-09T12:00:00Z",
    )
    .await;

    let response = send(
        &application,
        patch_json(
            &format!("/ledger/transactions/{}", transaction["id"].as_i64().unwrap()),
            json!({
                "category_id": category["id"],
                "amount_kopecks": -1200,
                "occurred_at": "2026-03-10T14:30:00+03:00"
            }),
            &cookie,
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_json_body(response).await,
        json!({"error": "occurred_at must be a UTC RFC 3339 timestamp ending in Z"})
    );
}

#[sqlx::test]
async fn user_deletes_transaction(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "ledger-transaction-delete@example.com", "UTC").await;
    let application = build_application(&pool);
    let category = create_category(&application, &cookie, "Food").await;
    let transaction = create_transaction(
        &application,
        &cookie,
        category["id"].as_i64().unwrap(),
        -900,
        "2026-03-09T12:00:00Z",
    )
    .await;

    let response = send(
        &application,
        delete_with_cookie(
            &format!("/ledger/transactions/{}", transaction["id"].as_i64().unwrap()),
            &cookie,
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let history = read_json_body(
        send(
            &application,
            build_history_request(&cookie, "2026-03-01T00:00:00Z", "2026-04-01T00:00:00Z"),
        )
        .await,
    )
    .await;
    assert_eq!(history["transactions"], json!([]));
}

#[sqlx::test]
async fn user_lists_only_their_transactions(pool: PgPool) {
    let (_, owner_cookie) = create_authenticated_user(&pool, "ledger-transactions-owner@example.com", "UTC").await;
    let (_, another_cookie) = create_authenticated_user(&pool, "ledger-transactions-another@example.com", "UTC").await;
    let application = build_application(&pool);
    let owner_category = create_category(&application, &owner_cookie, "Work").await;
    let another_category = create_category(&application, &another_cookie, "Private").await;
    let owner_transaction = create_transaction(
        &application,
        &owner_cookie,
        owner_category["id"].as_i64().unwrap(),
        150000,
        "2026-03-09T12:00:00Z",
    )
    .await;
    create_transaction(
        &application,
        &another_cookie,
        another_category["id"].as_i64().unwrap(),
        80000,
        "2026-03-09T12:00:00Z",
    )
    .await;

    let response = send(
        &application,
        build_history_request(&owner_cookie, "2026-03-01T00:00:00Z", "2026-04-01T00:00:00Z"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let history = read_json_body(response).await;
    assert_eq!(history["start_at"], "2026-03-01T00:00:00Z");
    assert_eq!(history["end_at"], "2026-04-01T00:00:00Z");
    let transactions = history["transactions"].as_array().unwrap();
    assert_eq!(transactions.len(), 1);
    assert_eq!(transactions[0]["id"], owner_transaction["id"]);
    assert_eq!(transactions[0]["category_name"], "Work");
}

#[sqlx::test]
async fn user_lists_explicit_utc_interval(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "ledger-utc-range@example.com", "Europe/London").await;
    let application = build_application(&pool);

    let response = send(
        &application,
        build_history_request(&cookie, "2026-03-01T00:00:00Z", "2026-03-31T23:00:00Z"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let history = read_json_body(response).await;
    assert_eq!(history["start_at"], "2026-03-01T00:00:00Z");
    assert_eq!(history["end_at"], "2026-03-31T23:00:00Z");
}

#[sqlx::test]
async fn user_sees_transactions_at_start_of_interval_but_not_end(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "ledger-exclusive-end@example.com", "Europe/London").await;
    let application = build_application(&pool);
    let category = create_category(&application, &cookie, "Work").await;
    let category_id = category["id"].as_i64().unwrap();
    create_transaction(&application, &cookie, category_id, 1000, "2026-02-28T23:59:59Z").await;
    let start_transaction = create_transaction(&application, &cookie, category_id, 100, "2026-03-01T00:00:00Z").await;
    let before_end_transaction =
        create_transaction(&application, &cookie, category_id, -200, "2026-03-31T22:59:59Z").await;
    create_transaction(&application, &cookie, category_id, 500, "2026-03-31T23:00:00Z").await;

    let response = send(
        &application,
        build_history_request(&cookie, "2026-03-01T00:00:00Z", "2026-03-31T23:00:00Z"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let history: Value = read_json_body(response).await;
    assert_eq!(history["start_at"], "2026-03-01T00:00:00Z");
    assert_eq!(history["end_at"], "2026-03-31T23:00:00Z");
    assert_eq!(history["balance_kopecks"], 900);
    let transactions = history["transactions"].as_array().unwrap();
    assert_eq!(transactions.len(), 2);
    assert_eq!(transactions[0]["id"], before_end_transaction["id"]);
    assert_eq!(transactions[1]["id"], start_transaction["id"]);
}

#[sqlx::test]
async fn user_cannot_list_transactions_with_invalid_interval_bounds(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "ledger-invalid-range@example.com", "UTC").await;
    let application = build_application(&pool);
    let invalid_ranges = [
        (
            "not-a-timestampZ",
            "2026-04-01T00:00:00Z",
            "start_at must be a valid RFC 3339 timestamp",
        ),
        (
            "2026-03-01T00:00:00Z",
            "not-a-timestampZ",
            "end_at must be a valid RFC 3339 timestamp",
        ),
        (
            "2026-03-01T00:00:00%2B03:00",
            "2026-04-01T00:00:00Z",
            "start_at must be a UTC RFC 3339 timestamp ending in Z",
        ),
        (
            "2026-03-01T00:00:00Z",
            "2026-04-01T00:00:00%2B03:00",
            "end_at must be a UTC RFC 3339 timestamp ending in Z",
        ),
        (
            "2026-03-01T00:00:00Z",
            "2026-03-01T00:00:00Z",
            "start_at must be before end_at",
        ),
        (
            "2026-04-01T00:00:00Z",
            "2026-03-01T00:00:00Z",
            "start_at must be before end_at",
        ),
    ];

    for (start_at, end_at, expected_error) in invalid_ranges {
        let response = send(&application, build_history_request(&cookie, start_at, end_at)).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(read_json_body(response).await["error"], expected_error);
    }
}

#[sqlx::test]
async fn unauthenticated_user_cannot_list_transactions(pool: PgPool) {
    let application = build_application(&pool);
    let response = send(
        &application,
        Request::get("/ledger/transactions?start_at=2026-03-01T00:00:00Z&end_at=2026-04-01T00:00:00Z")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

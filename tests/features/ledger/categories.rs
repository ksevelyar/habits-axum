use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;

use super::super::support::{
    build_application, create_authenticated_user, create_category, create_transaction, delete_with_cookie,
    get_with_cookie, patch_json, post_json, read_json_body, send,
};

#[sqlx::test]
async fn user_creates_category(pool: PgPool) {
    let (user_id, cookie) = create_authenticated_user(&pool, "ledger-category-create@example.com", "UTC").await;
    let application = build_application(&pool);

    let response = send(
        &application,
        post_json("/ledger/categories", json!({"name": "Work"}), &cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let category = read_json_body(response).await;
    assert_eq!(category["user_id"].as_i64().unwrap(), user_id);
    assert_eq!(category["name"], "Work");
    assert!(category["id"].as_i64().is_some());
    assert!(category["inserted_at"].as_str().unwrap().ends_with('Z'));
    assert!(category["updated_at"].as_str().unwrap().ends_with('Z'));
}

#[sqlx::test]
async fn authenticated_user_lists_their_categories(pool: PgPool) {
    let (owner_id, owner_cookie) = create_authenticated_user(&pool, "ledger-category-owner@example.com", "UTC").await;
    let (_, another_cookie) = create_authenticated_user(&pool, "ledger-category-another@example.com", "UTC").await;
    let application = build_application(&pool);
    create_category(&application, &owner_cookie, "Work").await;
    create_category(&application, &another_cookie, "Private category").await;

    let response = send(&application, get_with_cookie("/ledger/categories", &owner_cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let categories = read_json_body(response).await;
    let categories = categories.as_array().unwrap();
    assert_eq!(categories.len(), 1);
    assert_eq!(categories[0]["user_id"], owner_id);
    assert_eq!(categories[0]["name"], "Work");
    assert!(categories[0]["id"].as_i64().is_some());
    assert!(categories[0]["inserted_at"].as_str().unwrap().ends_with('Z'));
    assert!(categories[0]["updated_at"].as_str().unwrap().ends_with('Z'));
}

#[sqlx::test]
async fn user_renames_category(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "ledger-category-rename@example.com", "UTC").await;
    let application = build_application(&pool);
    let category = create_category(&application, &cookie, "House").await;
    let category_id = category["id"].as_i64().unwrap();

    let response = send(
        &application,
        patch_json(
            &format!("/ledger/categories/{category_id}"),
            json!({"name": "Home"}),
            &cookie,
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(read_json_body(response).await["name"], "Home");
}

#[sqlx::test]
async fn user_deletes_unused_category(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "ledger-category-delete@example.com", "UTC").await;
    let application = build_application(&pool);
    let category = create_category(&application, &cookie, "Unused").await;
    let category_id = category["id"].as_i64().unwrap();

    let response = send(
        &application,
        delete_with_cookie(&format!("/ledger/categories/{category_id}"), &cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let categories = read_json_body(send(&application, get_with_cookie("/ledger/categories", &cookie)).await).await;
    assert_eq!(categories, json!([]));
}

#[sqlx::test]
async fn user_cannot_delete_category_with_transactions(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "ledger-category-in-use@example.com", "UTC").await;
    let application = build_application(&pool);
    let category = create_category(&application, &cookie, "Home").await;
    create_transaction(
        &application,
        &cookie,
        category["id"].as_i64().unwrap(),
        -15990,
        "2026-03-09T12:00:00Z",
    )
    .await;

    let response = send(
        &application,
        delete_with_cookie(
            &format!("/ledger/categories/{}", category["id"].as_i64().unwrap()),
            &cookie,
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_json_body(response).await,
        json!({"error": "category has transactions"})
    );
}

#[sqlx::test]
async fn user_cannot_access_another_users_category(pool: PgPool) {
    let (_, owner_cookie) = create_authenticated_user(&pool, "ledger-category-visible@example.com", "UTC").await;
    let (_, another_cookie) = create_authenticated_user(&pool, "ledger-category-hidden@example.com", "UTC").await;
    let application = build_application(&pool);
    let visible_category = create_category(&application, &owner_cookie, "Work").await;
    let hidden_category = create_category(&application, &another_cookie, "Private").await;
    let hidden_category_id = hidden_category["id"].as_i64().unwrap();

    let response = send(&application, get_with_cookie("/ledger/categories", &owner_cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let visible_categories = read_json_body(response).await;
    let visible_categories = visible_categories.as_array().unwrap();
    assert_eq!(visible_categories.len(), 1);
    assert_eq!(visible_categories[0]["id"], visible_category["id"]);

    let foreign_update_response = send(
        &application,
        patch_json(
            &format!("/ledger/categories/{hidden_category_id}"),
            json!({"name": "Taken over"}),
            &owner_cookie,
        ),
    )
    .await;
    assert_eq!(foreign_update_response.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn unauthenticated_user_cannot_access_categories(pool: PgPool) {
    let application = build_application(&pool);
    let response = send(
        &application,
        axum::http::Request::get("/ledger/categories")
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;

use super::support::{
    build_application, create_authenticated_user, get_with_cookie, patch_json, post_json, read_json_body, send,
};

#[sqlx::test]
async fn user_creates_active_reminder(pool: PgPool) {
    let (user_id, cookie) = create_authenticated_user(&pool, "task@example.com", "Europe/London").await;
    let application = build_application(&pool);
    let response = send(
        &application,
        post_json(
            "/tasks",
            json!({"name":"Take a walk","cron":"0 18 * * *","active":true}),
            &cookie,
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let task = read_json_body(response).await;
    assert!(task["id"].as_i64().is_some());
    assert_eq!(task["name"], "Take a walk");
    assert_eq!(task["active"], true);
    assert_eq!(task["cron"], "0 18 * * *");
    assert_eq!(task["user_id"], user_id);
    assert!(task["inserted_at"].as_str().unwrap().ends_with('Z'));
    assert!(task["updated_at"].as_str().unwrap().ends_with('Z'));
}

#[sqlx::test]
async fn user_updates_reminder(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "task@example.com", "Europe/London").await;
    let application = build_application(&pool);
    let task = read_json_body(
        send(
            &application,
            post_json(
                "/tasks",
                json!({"name":"Take a walk","cron":"0 18 * * *","active":true}),
                &cookie,
            ),
        )
        .await,
    )
    .await;
    let response = send(
        &application,
        patch_json(&format!("/tasks/{}", task["id"]), json!({"active":false}), &cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated_task = read_json_body(response).await;
    assert_eq!(updated_task["name"], "Take a walk");
    assert_eq!(updated_task["active"], false);
    assert_eq!(updated_task["cron"], "0 18 * * *");
}

#[sqlx::test]
async fn user_cannot_create_task_without_cron_expression(pool: PgPool) {
    let (_, cookie) = create_authenticated_user(&pool, "task@example.com", "Europe/London").await;
    let application = build_application(&pool);
    let response = send(
        &application,
        post_json("/tasks", json!({"name":"Take a walk"}), &cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        read_json_body(response).await,
        json!({"error":"validation failed","fields":[{"field":"cron","message":"cron is required"}]})
    );
}

#[sqlx::test]
async fn user_cannot_access_another_users_task(pool: PgPool) {
    let (_, owner_cookie) = create_authenticated_user(&pool, "owner@example.com", "Europe/London").await;
    let (_, another_cookie) = create_authenticated_user(&pool, "another@example.com", "Europe/London").await;
    let application = build_application(&pool);
    let task = read_json_body(
        send(
            &application,
            post_json(
                "/tasks",
                json!({"name":"Take a walk","cron":"0 18 * * *","active":true}),
                &owner_cookie,
            ),
        )
        .await,
    )
    .await;
    let response = send(
        &application,
        get_with_cookie(&format!("/tasks/{}", task["id"]), &another_cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(read_json_body(response).await, json!({"error":"task not found"}));
}

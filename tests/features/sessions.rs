use axum::http::StatusCode;
use cookie::Cookie;
use serde_json::json;
use sqlx::PgPool;

use super::support::{
    build_application, create_user_with_details, get_with_cookie, parse_cookie, post_json_without_cookie,
    read_json_body, send,
};

#[sqlx::test]
async fn user_signs_in(pool: PgPool) {
    create_user_with_details(
        &pool,
        "person@example.com",
        "correct horse battery staple",
        "Europe/London",
        "person",
    )
    .await;
    let response = send(
        &build_application(&pool),
        post_json_without_cookie(
            "/sessions",
            json!({"email":"person@example.com","password":"correct horse battery staple"}),
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let cookie = Cookie::parse(response.headers().get("set-cookie").unwrap().to_str().unwrap()).unwrap();
    assert_eq!(cookie.name(), "jwt");
    assert!(cookie.http_only().unwrap());
    assert_eq!(cookie.path(), Some("/"));
    assert_eq!(cookie.max_age().unwrap().whole_seconds(), 31_536_000);
}

#[sqlx::test]
async fn signed_in_user_reads_their_account(pool: PgPool) {
    create_user_with_details(
        &pool,
        "person@example.com",
        "correct horse battery staple",
        "Europe/London",
        "person",
    )
    .await;
    let application = build_application(&pool);
    let session = send(
        &application,
        post_json_without_cookie(
            "/sessions",
            json!({"email":"person@example.com","password":"correct horse battery staple"}),
        ),
    )
    .await;
    let response = send(
        &application,
        get_with_cookie("/sessions/current", &parse_cookie(&session)),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let user = read_json_body(response).await;
    assert!(user["id"].as_i64().is_some());
    assert_eq!(user["email"], "person@example.com");
    assert_eq!(user["timezone"], "Europe/London");
    assert!(user.get("password").is_none());
    assert!(user.get("password_hash").is_none());
}

#[sqlx::test]
async fn user_cannot_sign_in_with_invalid_credentials(pool: PgPool) {
    create_user_with_details(
        &pool,
        "person@example.com",
        "correct horse battery staple",
        "Europe/London",
        "person",
    )
    .await;
    let response = send(
        &build_application(&pool),
        post_json_without_cookie(
            "/sessions",
            json!({"email":"person@example.com","password":"incorrect password"}),
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(read_json_body(response).await, json!({"error":"unauthorized"}));
}

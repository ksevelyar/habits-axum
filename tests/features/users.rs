use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;

use super::support::{build_application, post_json_without_cookie, read_json_body, send};

#[sqlx::test]
async fn user_registers_account(pool: PgPool) {
    let application = build_application(&pool);
    let response = send(
        &application,
        post_json_without_cookie(
            "/users",
            json!({"email":"person@example.com","handle":"person","password":"correct horse battery staple","timezone":"Europe/London"}),
        ),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let user = read_json_body(response).await;
    assert!(user["id"].as_i64().is_some());
    assert_eq!(user["email"], "person@example.com");
    assert_eq!(user["timezone"], "Europe/London");
    assert!(user.get("password").is_none());
    assert!(user.get("password_hash").is_none());
}

#[sqlx::test]
async fn user_cannot_register_with_email_already_in_use(pool: PgPool) {
    let application = build_application(&pool);
    let account = json!({"email":"person@example.com","handle":"person","password":"correct horse battery staple","timezone":"Europe/London"});
    assert_eq!(
        send(&application, post_json_without_cookie("/users", account))
            .await
            .status(),
        StatusCode::CREATED
    );

    let response = send(&application, post_json_without_cookie("/users", json!({"email":"PERSON@example.com","handle":"another-person","password":"correct horse battery staple","timezone":"Europe/London"}))).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        read_json_body(response).await,
        json!({"error":"validation failed","fields":[{"field":"email","message":"email is already taken"}]})
    );
}

#[sqlx::test]
async fn user_cannot_register_with_unknown_timezone(pool: PgPool) {
    let application = build_application(&pool);
    let response = send(&application, post_json_without_cookie("/users", json!({"email":"person@example.com","handle":"person","password":"correct horse battery staple","timezone":"Mars/Olympus_Mons"}))).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        read_json_body(response).await,
        json!({"error":"validation failed","fields":[{"field":"timezone","message":"unknown timezone"}]})
    );
}

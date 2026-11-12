use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use cookie::Cookie;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::{Arc, Once};
use tower::ServiceExt;
use tracing_subscriber::EnvFilter;

use habits_axum::authentication::encode_jwt;
use habits_axum::build_app;
use habits_axum::clock::SystemClock;

pub fn initialize_test_tracing() {
    static TEST_TRACING_INITIALIZER: Once = Once::new();

    TEST_TRACING_INITIALIZER.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("error")))
            .with_test_writer()
            .init();
    });
}

pub async fn read_json_body(response: Response<Body>) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

pub fn parse_cookie(response: &Response<Body>) -> String {
    let cookie_header = response.headers().get("set-cookie").unwrap().to_str().unwrap();
    let cookie = Cookie::parse(cookie_header).unwrap();
    format!("{}={}", cookie.name(), cookie.value())
}

pub async fn create_authenticated_user(pool: &PgPool, email: &str, timezone: &str) -> (i64, String) {
    let user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (email, password_hash, handle, timezone) VALUES ($1, 'unused', $1, $2) RETURNING id",
    )
    .bind(email)
    .bind(timezone)
    .fetch_one(pool)
    .await
    .unwrap();
    let session_token = encode_jwt(email.to_owned()).unwrap();
    (user_id, format!("jwt={session_token}"))
}

pub fn build_application(pool: &PgPool) -> Router {
    initialize_test_tracing();
    build_app(pool.clone(), None, Arc::new(SystemClock))
}

pub async fn send(application: &Router, request: Request<Body>) -> Response<Body> {
    application.clone().oneshot(request).await.unwrap()
}

pub fn post_json_without_cookie(uri: &str, body: Value) -> Request<Body> {
    Request::post(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

pub fn post_json(uri: &str, body: Value, cookie: &str) -> Request<Body> {
    Request::post(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie)
        .body(Body::from(body.to_string()))
        .unwrap()
}

pub fn patch_json(uri: &str, body: Value, cookie: &str) -> Request<Body> {
    Request::patch(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie)
        .body(Body::from(body.to_string()))
        .unwrap()
}

pub fn get_request(uri: &str) -> Request<Body> {
    Request::get(uri).body(Body::empty()).unwrap()
}

pub fn get_with_cookie(uri: &str, cookie: &str) -> Request<Body> {
    Request::get(uri)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

pub fn delete_with_cookie(uri: &str, cookie: &str) -> Request<Body> {
    Request::delete(uri)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

pub async fn create_user(pool: &PgPool, email: &str) {
    create_user_with_details(pool, email, "x", "Europe/London", email).await;
}

pub async fn create_user_with_details(pool: &PgPool, email: &str, password: &str, timezone: &str, handle: &str) {
    let response = send(
        &build_application(pool),
        post_json_without_cookie(
            "/users",
            json!({
                "email": email,
                "password": password,
                "timezone": timezone,
                "handle": handle,
            }),
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

pub async fn create_session(pool: &PgPool, email: &str) -> String {
    create_session_with_password(pool, email, "x").await
}

pub async fn create_session_with_password(pool: &PgPool, email: &str, password: &str) -> String {
    let response = send(
        &build_application(pool),
        post_json_without_cookie("/sessions", json!({"email": email, "password": password})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    parse_cookie(&response)
}

pub async fn create_user_and_session(
    pool: &PgPool,
    email: &str,
    password: &str,
    timezone: &str,
    handle: &str,
) -> String {
    create_user_with_details(pool, email, password, timezone, handle).await;
    create_session_with_password(pool, email, password).await
}

pub async fn create_category(application: &Router, cookie: &str, name: &str) -> Value {
    let response = send(
        application,
        post_json("/ledger/categories", json!({"name": name}), cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    read_json_body(response).await
}

pub async fn create_transaction(
    application: &Router,
    cookie: &str,
    category_id: i64,
    amount_kopecks: i64,
    occurred_at: &str,
) -> Value {
    let response = send(
        application,
        post_json(
            "/ledger/transactions",
            json!({
                "category_id": category_id,
                "amount_kopecks": amount_kopecks,
                "occurred_at": occurred_at,
            }),
            cookie,
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    read_json_body(response).await
}

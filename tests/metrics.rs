use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use cookie::Cookie;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::{Service, ServiceExt};

use habits_axum::app;

async fn json_body(res: Response<Body>) -> Value {
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn cookie_from(res: &Response<Body>) -> String {
    let s = res.headers().get("set-cookie").unwrap().to_str().unwrap();
    let c = Cookie::parse(s).unwrap();
    format!("{}={}", c.name(), c.value())
}

fn post_json(uri: &str, body: Value) -> Request<Body> {
    Request::post(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn create_user(pool: &PgPool, email: &str) {
    let mut app = app(pool.clone()).into_service();
    let res = app
        .ready()
        .await
        .unwrap()
        .call(post_json(
            "/users",
            json!({"email": email, "password": "x", "timezone": "Europe/London", "handle": email}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
}

async fn session(pool: &PgPool, email: &str) -> String {
    let mut app = app(pool.clone()).into_service();
    let res = app
        .ready()
        .await
        .unwrap()
        .call(post_json("/sessions", json!({"email": email, "password": "x"})))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    cookie_from(&res)
}

async fn device_token(pool: &PgPool, cookie: &str) -> String {
    let mut app = app(pool.clone()).into_service();
    let mut req = post_json("/devices", json!({"device_name": "esp32-stepper"}));
    req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
    let res = app.ready().await.unwrap().call(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    json_body(res).await["token"].as_str().unwrap().to_string()
}

async fn create_time_chain(pool: &PgPool, cookie: &str) -> i64 {
    let mut app = app(pool.clone()).into_service();
    let mut req = post_json(
        "/chains",
        json!({
            "active": true,
            "name": "walking",
            "type": "time",
            "aggregate": "sum",
        }),
    );
    req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
    let res = app.ready().await.unwrap().call(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    json_body(res).await["id"].as_i64().unwrap()
}

fn post_metric(uri: &str, body: Value, token: &str) -> Request<Body> {
    Request::post(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[sqlx::test]
async fn upsert_metric_with_device_token(pool: PgPool) {
    create_user(&pool, "t@t.com").await;
    let cookie = session(&pool, "t@t.com").await;
    let token = device_token(&pool, &cookie).await;
    let chain_id = create_time_chain(&pool, &cookie).await;
    let mut app = app(pool.clone()).into_service();

    let res = app
        .ready()
        .await
        .unwrap()
        .call(post_metric(
            "/metrics",
            json!({"date": "2026-08-09", "value": "125", "chain_id": chain_id}),
            &token,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = json_body(res).await;
    assert_eq!(body["chain_id"], chain_id);
    assert_eq!(body["value_integer"], 125);

    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM metrics")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test]
async fn upsert_metric_replaces_same_date(pool: PgPool) {
    create_user(&pool, "t@t.com").await;
    let cookie = session(&pool, "t@t.com").await;
    let token = device_token(&pool, &cookie).await;
    let chain_id = create_time_chain(&pool, &cookie).await;
    let mut app = app(pool.clone()).into_service();

    for value in ["60", "90"] {
        let res = app
            .ready()
            .await
            .unwrap()
            .call(post_metric(
                "/metrics",
                json!({"date": "2026-08-09", "value": value, "chain_id": chain_id}),
                &token,
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM metrics")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);

    let (value,): (i32,) = sqlx::query_as("SELECT value_integer FROM metrics")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(value, 90);
}

#[sqlx::test]
async fn upsert_metric_rejects_foreign_chain(pool: PgPool) {
    create_user(&pool, "a@t.com").await;
    create_user(&pool, "b@t.com").await;
    let cookie_a = session(&pool, "a@t.com").await;
    let cookie_b = session(&pool, "b@t.com").await;
    let token_b = device_token(&pool, &cookie_b).await;
    let chain_id = create_time_chain(&pool, &cookie_a).await;
    let mut app = app(pool.clone()).into_service();

    let res = app
        .ready()
        .await
        .unwrap()
        .call(post_metric(
            "/metrics",
            json!({"date": "2026-08-09", "value": "125", "chain_id": chain_id}),
            &token_b,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn upsert_metric_with_cookie_still_works(pool: PgPool) {
    create_user(&pool, "t@t.com").await;
    let cookie = session(&pool, "t@t.com").await;
    let chain_id = create_time_chain(&pool, &cookie).await;
    let mut app = app(pool.clone()).into_service();

    let mut req = post_json(
        "/metrics",
        json!({"date": "2026-08-09", "value": "125", "chain_id": chain_id}),
    );
    req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
    let res = app.ready().await.unwrap().call(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[sqlx::test]
async fn upsert_metric_without_auth_is_unauthorized(pool: PgPool) {
    create_user(&pool, "t@t.com").await;
    let cookie = session(&pool, "t@t.com").await;
    let chain_id = create_time_chain(&pool, &cookie).await;
    let mut app = app(pool.clone()).into_service();

    let res = app
        .ready()
        .await
        .unwrap()
        .call(post_json(
            "/metrics",
            json!({"date": "2026-08-09", "value": "125", "chain_id": chain_id}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

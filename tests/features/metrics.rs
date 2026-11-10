use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use jiff_sqlx::ToSqlx;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tower::{Service, ServiceExt};

use super::support::{create_session, create_user, get_with_cookie, post_json_without_cookie, read_json_body};
use habits_axum::build_app;
use habits_axum::clock::SystemClock;

async fn device_token(pool: &PgPool, cookie: &str) -> String {
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();
    let mut req = post_json_without_cookie("/devices", json!({"device_name": "esp32-stepper"}));
    req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
    let res = app.ready().await.unwrap().call(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    read_json_body(res).await["token"].as_str().unwrap().to_string()
}

async fn create_time_chain(pool: &PgPool, cookie: &str) -> i64 {
    create_chain(pool, cookie, "time").await
}

async fn create_chain(pool: &PgPool, cookie: &str, chain_type: &str) -> i64 {
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();
    let mut request = post_json_without_cookie(
        "/chains",
        json!({
            "active": true,
            "name": format!("{chain_type} metric"),
            "type": chain_type,
            "aggregate": "sum",
        }),
    );
    request.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
    let response = app.ready().await.unwrap().call(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    read_json_body(response).await["id"].as_i64().unwrap()
}

fn post_metric(uri: &str, body: Value, token: &str) -> Request<Body> {
    Request::post(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[sqlx::test]
async fn serialize_metric_dates_in_history(pool: PgPool) {
    create_user(&pool, "metric-history@example.com").await;
    let cookie = create_session(&pool, "metric-history@example.com").await;
    let chain_id = create_chain(&pool, &cookie, "integer").await;
    let metric_date = Timestamp::now().to_zoned(TimeZone::UTC).date();

    sqlx::query("INSERT INTO metrics (chain_id, date, value_integer) VALUES ($1, $2, $3)")
        .bind(chain_id)
        .bind(metric_date.to_sqlx())
        .bind(12_i32)
        .execute(&pool)
        .await
        .unwrap();

    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();
    let response = app
        .ready()
        .await
        .unwrap()
        .call(get_with_cookie("/metrics_history", &cookie))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let history = read_json_body(response).await;
    let metric_date_key = metric_date.to_string();
    let chain_id_key = chain_id.to_string();
    let metric = history["sprints"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|sprint| sprint["week"].get(&metric_date_key))
        .and_then(|metrics_by_chain| metrics_by_chain.get(&chain_id_key));

    assert_eq!(metric.unwrap()["date"], metric_date_key);

    let metric_response = app
        .ready()
        .await
        .unwrap()
        .call(get_with_cookie(&format!("/metrics?date={metric_date_key}"), &cookie))
        .await
        .unwrap();
    assert_eq!(metric_response.status(), StatusCode::OK);

    let metrics_by_date = read_json_body(metric_response).await;
    assert!(metrics_by_date[0]["updated_at"].as_str().unwrap().ends_with('Z'));
}

#[sqlx::test]
async fn upsert_metric_with_device_token(pool: PgPool) {
    create_user(&pool, "t@t.com").await;
    let cookie = create_session(&pool, "t@t.com").await;
    let token = device_token(&pool, &cookie).await;
    let chain_id = create_time_chain(&pool, &cookie).await;
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

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

    let body = read_json_body(res).await;
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
    let cookie = create_session(&pool, "t@t.com").await;
    let token = device_token(&pool, &cookie).await;
    let chain_id = create_time_chain(&pool, &cookie).await;
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

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
async fn reject_zero_empty_and_false_metric_values(pool: PgPool) {
    create_user(&pool, "metric-validation@example.com").await;
    let cookie = create_session(&pool, "metric-validation@example.com").await;
    let token = device_token(&pool, &cookie).await;
    let integer_chain_id = create_chain(&pool, &cookie, "integer").await;
    let float_chain_id = create_chain(&pool, &cookie, "float").await;
    let boolean_chain_id = create_chain(&pool, &cookie, "boolean").await;
    let time_chain_id = create_chain(&pool, &cookie, "time").await;
    let rejected_values = [
        (integer_chain_id, "0"),
        (float_chain_id, "0"),
        (integer_chain_id, ""),
        (boolean_chain_id, "false"),
        (time_chain_id, "0"),
    ];
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

    for (chain_id, value) in rejected_values {
        let response = app
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
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    let valid_values = [
        (integer_chain_id, "12"),
        (float_chain_id, "1.5"),
        (boolean_chain_id, "true"),
        (time_chain_id, "30"),
    ];

    for (chain_id, value) in valid_values {
        let response = app
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
        assert_eq!(response.status(), StatusCode::OK);
    }

    for (chain_id, value) in rejected_values {
        let response = app
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
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    let metric_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM metrics")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(metric_count, 4);

    let integer_value: i32 = sqlx::query_scalar("SELECT value_integer FROM metrics WHERE chain_id = $1")
        .bind(integer_chain_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let float_value: f64 = sqlx::query_scalar("SELECT value_float FROM metrics WHERE chain_id = $1")
        .bind(float_chain_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let boolean_value: bool = sqlx::query_scalar("SELECT value_bool FROM metrics WHERE chain_id = $1")
        .bind(boolean_chain_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let time_value: i32 = sqlx::query_scalar("SELECT value_integer FROM metrics WHERE chain_id = $1")
        .bind(time_chain_id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(integer_value, 12);
    assert_eq!(float_value, 1.5);
    assert!(boolean_value);
    assert_eq!(time_value, 30);
}

#[sqlx::test]
async fn upsert_metric_rejects_foreign_chain(pool: PgPool) {
    create_user(&pool, "a@t.com").await;
    create_user(&pool, "b@t.com").await;
    let cookie_a = create_session(&pool, "a@t.com").await;
    let cookie_b = create_session(&pool, "b@t.com").await;
    let token_b = device_token(&pool, &cookie_b).await;
    let chain_id = create_time_chain(&pool, &cookie_a).await;
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

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
    let cookie = create_session(&pool, "t@t.com").await;
    let chain_id = create_time_chain(&pool, &cookie).await;
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

    let mut req = post_json_without_cookie(
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
    let cookie = create_session(&pool, "t@t.com").await;
    let chain_id = create_time_chain(&pool, &cookie).await;
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

    let res = app
        .ready()
        .await
        .unwrap()
        .call(post_json_without_cookie(
            "/metrics",
            json!({"date": "2026-08-09", "value": "125", "chain_id": chain_id}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

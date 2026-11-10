use axum::http::{StatusCode, header};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tower::{Service, ServiceExt};

use super::support::{create_user_and_session, post_json_without_cookie, read_json_body};
use habits_axum::authentication;
use habits_axum::build_app;
use habits_axum::clock::SystemClock;

#[sqlx::test]
async fn create_device(pool: PgPool) {
    let cookie = create_user_and_session(&pool, "t@t.com", "x", "Europe/London", "user9000").await;
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

    let mut req = post_json_without_cookie("/devices", json!({"device_name": "esp32-display"}));
    req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
    let res = app.ready().await.unwrap().call(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let body = read_json_body(res).await;
    let device_id = body["device_id"].as_str().unwrap().to_string();
    assert_eq!(body["device_name"], "esp32-display");
    let token = body["token"].as_str().unwrap().to_string();

    let token_data = authentication::decode_jwt(&token).unwrap();
    assert_eq!(token_data.claims.email, "t@t.com");
    assert_eq!(token_data.claims.device_id, Some(device_id));
    assert_eq!(token_data.claims.device_name, Some("esp32-display".to_string()));
    assert_eq!(token_data.claims.exp, u64::MAX);
}

#[sqlx::test]
async fn create_device_with_invalid_params(pool: PgPool) {
    let cookie = create_user_and_session(&pool, "t@t.com", "x", "Europe/London", "user9000").await;
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

    for payload in [json!({}), json!("not_an_object")] {
        let mut req = post_json_without_cookie("/devices", payload);
        req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
        let res = app.ready().await.unwrap().call(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}

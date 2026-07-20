use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use cookie::Cookie;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::{Service, ServiceExt};

use habits_axum::app;
use habits_axum::authentication;

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

async fn session(pool: &PgPool) -> String {
    let mut app = app(pool.clone()).into_service();

    let create_user_response = app
        .ready()
        .await
        .unwrap()
        .call(post_json(
            "/users",
            json!({"email": "t@t.com", "password": "x", "timezone": "Europe/London", "handle": "user9000"}),
        ))
        .await
        .unwrap();
    assert_eq!(create_user_response.status(), StatusCode::CREATED);

    let create_session_response = app
        .ready()
        .await
        .unwrap()
        .call(post_json("/sessions", json!({"email": "t@t.com", "password": "x"})))
        .await
        .unwrap();
    assert_eq!(create_session_response.status(), StatusCode::CREATED);

    cookie_from(&create_session_response)
}

#[sqlx::test]
async fn create_device(pool: PgPool) {
    let cookie = session(&pool).await;
    let mut app = app(pool.clone()).into_service();

    let mut req = post_json("/devices", json!({"device_name": "esp32-display"}));
    req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
    let res = app.ready().await.unwrap().call(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let body = json_body(res).await;
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
    let cookie = session(&pool).await;
    let mut app = app(pool.clone()).into_service();

    for payload in [json!({}), json!("not_an_object")] {
        let mut req = post_json("/devices", payload);
        req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
        let res = app.ready().await.unwrap().call(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}

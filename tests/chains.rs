use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use cookie::Cookie;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::{Service, ServiceExt};

use habits_axum::app;
use habits_axum::chains::Chain;

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

fn patch_json(uri: &str, body: Value, cookie: &str) -> Request<Body> {
    Request::patch(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie)
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get_req(uri: &str, cookie: &str) -> Request<Body> {
    Request::get(uri)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

fn delete_req(uri: &str, cookie: &str) -> Request<Body> {
    Request::delete(uri)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
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
async fn create_integer_chain(pool: PgPool) {
    let cookie = session(&pool).await;
    let mut app = app(pool.clone()).into_service();

    let res = {
        let mut req = post_json(
            "/chains",
            json!({
                "active": true,
                "name": "workout",
                "type": "integer",
                "aggregate": "sum",
                "description": "kettlebell + pull ups",
                "order": 5,
            }),
        );
        req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
        app.ready().await.unwrap().call(req).await.unwrap()
    };
    assert_eq!(res.status(), StatusCode::CREATED);

    let body = json_body(res).await;
    body["id"].as_i64().unwrap();
    assert_eq!(body["active"], true);
    assert_eq!(body["name"], "workout");
    assert_eq!(body["type"], "integer");
    assert_eq!(body["aggregate"], "sum");
    assert_eq!(body["description"], "kettlebell + pull ups");
    assert_eq!(body["order"], 5);
    assert!(body["inserted_at"].as_str().is_some());
}

#[sqlx::test]
async fn create_float_chain(pool: PgPool) {
    let cookie = session(&pool).await;
    let mut app = app(pool.clone()).into_service();

    let res = {
        let mut req = post_json(
            "/chains",
            json!({
                "active": true,
                "name": "weight",
                "type": "float",
                "aggregate": "avg",
            }),
        );
        req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
        app.ready().await.unwrap().call(req).await.unwrap()
    };
    assert_eq!(res.status(), StatusCode::CREATED);

    let body = json_body(res).await;
    assert_eq!(body["type"], "float");
    assert_eq!(body["name"], "weight");
    assert_eq!(body["aggregate"], "avg");
}

#[sqlx::test]
async fn create_boolean_chain(pool: PgPool) {
    let cookie = session(&pool).await;
    let mut app = app(pool.clone()).into_service();

    let res = {
        let mut req = post_json(
            "/chains",
            json!({
                "active": true,
                "name": "meditation",
                "type": "boolean",
                "aggregate": "sum",
            }),
        );
        req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
        app.ready().await.unwrap().call(req).await.unwrap()
    };
    assert_eq!(res.status(), StatusCode::CREATED);

    let body = json_body(res).await;
    assert_eq!(body["type"], "boolean");
    assert_eq!(body["name"], "meditation");
}

#[sqlx::test]
async fn create_time_chain(pool: PgPool) {
    let cookie = session(&pool).await;
    let mut app = app(pool.clone()).into_service();

    let res = {
        let mut req = post_json(
            "/chains",
            json!({
                "active": true,
                "name": "reading",
                "type": "time",
                "aggregate": "sum",
            }),
        );
        req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
        app.ready().await.unwrap().call(req).await.unwrap()
    };
    assert_eq!(res.status(), StatusCode::CREATED);

    let body = json_body(res).await;
    assert_eq!(body["type"], "time");
    assert_eq!(body["name"], "reading");
}

#[sqlx::test]
async fn create_chain_with_invalid_payload(pool: PgPool) {
    let cookie = session(&pool).await;
    let mut app = app(pool.clone()).into_service();

    for payload in [
        json!({"active": true}),
        json!({"name": "x"}),
        json!({"active": true, "name": "x", "type": "bad", "aggregate": "sum"}),
        json!("not_an_object"),
    ] {
        let mut req = post_json("/chains", payload);
        req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
        let res = app.ready().await.unwrap().call(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[sqlx::test]
async fn update_chain(pool: PgPool) {
    let cookie = session(&pool).await;
    let mut app = app(pool.clone()).into_service();

    let mut req = post_json(
        "/chains",
        json!({
            "active": true,
            "name": "original",
            "type": "integer",
            "aggregate": "sum",
            "description": "original desc",
            "order": 1,
        }),
    );
    req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
    let res = app.ready().await.unwrap().call(req).await.unwrap();
    let id = json_body(res).await["id"].as_i64().unwrap();

    let res = app
        .ready()
        .await
        .unwrap()
        .call(patch_json(
            &format!("/chains/{id}"),
            json!({
                "name": "updated",
                "active": false,
                "order": 10,
            }),
            &cookie,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = json_body(res).await;
    assert_eq!(body["name"], "updated");
    assert_eq!(body["active"], false);
    assert_eq!(body["order"], 10);
    assert_eq!(body["description"], "original desc");
    assert_eq!(body["type"], "integer");
    assert_eq!(body["aggregate"], "sum");

    let row = sqlx::query_as::<_, Chain>("SELECT * FROM chains WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.name, "updated");
    assert!(!row.active);
    assert_eq!(row.order, Some(10));
    assert_eq!(row.description.as_deref(), Some("original desc"));
}

#[sqlx::test]
async fn update_chain_with_invalid_payload(pool: PgPool) {
    let cookie = session(&pool).await;
    let mut app = app(pool.clone()).into_service();

    let mut req = post_json(
        "/chains",
        json!({
            "active": true, "name": "x", "type": "integer", "aggregate": "sum",
        }),
    );
    req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
    let res = app.ready().await.unwrap().call(req).await.unwrap();
    let id = json_body(res).await["id"].as_i64().unwrap();

    let res = app
        .ready()
        .await
        .unwrap()
        .call(patch_json(&format!("/chains/{id}"), json!({"type": "bad"}), &cookie))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn delete_chain(pool: PgPool) {
    let cookie = session(&pool).await;
    let mut app = app(pool.clone()).into_service();

    let mut req = post_json(
        "/chains",
        json!({
            "active": true, "name": "x", "type": "integer", "aggregate": "sum",
        }),
    );
    req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
    let res = app.ready().await.unwrap().call(req).await.unwrap();
    let id = json_body(res).await["id"].as_i64().unwrap();

    let res = app
        .ready()
        .await
        .unwrap()
        .call(delete_req(&format!("/chains/{id}"), &cookie))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM chains WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test]
async fn show_chain(pool: PgPool) {
    let cookie = session(&pool).await;
    let mut app = app(pool.clone()).into_service();

    let mut req = post_json(
        "/chains",
        json!({
            "active": false,
            "name": "inactive habit",
            "type": "boolean",
            "aggregate": "avg",
            "description": "hidden",
            "order": 3,
        }),
    );
    req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
    let res = app.ready().await.unwrap().call(req).await.unwrap();
    let created = json_body(res).await;
    let id = created["id"].as_i64().unwrap();

    let res = app
        .ready()
        .await
        .unwrap()
        .call(get_req(&format!("/chains/{id}"), &cookie))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(json_body(res).await, created);
}

#[sqlx::test]
async fn show_chains(pool: PgPool) {
    let cookie = session(&pool).await;
    let mut app = app(pool.clone()).into_service();

    let items = vec![
        json!({"active": true, "name": "a", "type": "integer", "aggregate": "sum"}),
        json!({"active": true, "name": "b", "type": "float", "aggregate": "avg"}),
        json!({"active": false, "name": "c", "type": "boolean", "aggregate": "sum"}),
    ];

    for payload in &items {
        let mut req = post_json("/chains", payload.clone());
        req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
        let res = app.ready().await.unwrap().call(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    let res = app
        .ready()
        .await
        .unwrap()
        .call(get_req("/chains", &cookie))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let list = json_body(res).await;
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0]["name"], "c");
    assert_eq!(list[1]["name"], "b");
    assert_eq!(list[2]["name"], "a");
}

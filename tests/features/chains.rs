use axum::http::{StatusCode, header};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tower::{Service, ServiceExt};

use super::support::{
    create_user_and_session, delete_with_cookie, get_with_cookie, patch_json, post_json_without_cookie, read_json_body,
};
use habits_axum::build_app;
use habits_axum::chains::Chain;
use habits_axum::clock::SystemClock;

async fn create_test_session(pool: &PgPool) -> String {
    create_user_and_session(pool, "t@t.com", "x", "Europe/London", "user9000").await
}

#[sqlx::test]
async fn create_integer_chain(pool: PgPool) {
    let cookie = create_test_session(&pool).await;
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

    let res = {
        let mut req = post_json_without_cookie(
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

    let body = read_json_body(res).await;
    body["id"].as_i64().unwrap();
    assert_eq!(body["active"], true);
    assert_eq!(body["show_in_charts"], false);
    assert_eq!(body["name"], "workout");
    assert_eq!(body["type"], "integer");
    assert_eq!(body["aggregate"], "sum");
    assert_eq!(body["description"], "kettlebell + pull ups");
    assert_eq!(body["order"], 5);
    assert!(body["inserted_at"].as_str().unwrap().ends_with('Z'));
}

#[sqlx::test]
async fn create_float_chain(pool: PgPool) {
    let cookie = create_test_session(&pool).await;
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

    let res = {
        let mut req = post_json_without_cookie(
            "/chains",
            json!({
                "active": true,
                "name": "weight",
                "type": "float",
                "aggregate": "avg",
                "show_in_charts": true,
            }),
        );
        req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
        app.ready().await.unwrap().call(req).await.unwrap()
    };
    assert_eq!(res.status(), StatusCode::CREATED);

    let body = read_json_body(res).await;
    assert_eq!(body["type"], "float");
    assert_eq!(body["show_in_charts"], true);
    assert_eq!(body["name"], "weight");
    assert_eq!(body["aggregate"], "avg");
}

#[sqlx::test]
async fn create_boolean_chain(pool: PgPool) {
    let cookie = create_test_session(&pool).await;
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

    let res = {
        let mut req = post_json_without_cookie(
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

    let body = read_json_body(res).await;
    assert_eq!(body["type"], "boolean");
    assert_eq!(body["name"], "meditation");
}

#[sqlx::test]
async fn create_time_chain(pool: PgPool) {
    let cookie = create_test_session(&pool).await;
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

    let res = {
        let mut req = post_json_without_cookie(
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

    let body = read_json_body(res).await;
    assert_eq!(body["type"], "time");
    assert_eq!(body["name"], "reading");
}

#[sqlx::test]
async fn create_chain_with_invalid_payload(pool: PgPool) {
    let cookie = create_test_session(&pool).await;
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

    for payload in [
        json!({"active": true}),
        json!({"name": "x"}),
        json!({"active": true, "name": "x", "type": "bad", "aggregate": "sum"}),
        json!("not_an_object"),
    ] {
        let mut req = post_json_without_cookie("/chains", payload);
        req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
        let res = app.ready().await.unwrap().call(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[sqlx::test]
async fn update_chain(pool: PgPool) {
    let cookie = create_test_session(&pool).await;
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

    let mut req = post_json_without_cookie(
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
    let id = read_json_body(res).await["id"].as_i64().unwrap();

    let res = app
        .ready()
        .await
        .unwrap()
        .call(patch_json(
            &format!("/chains/{id}"),
            json!({
                "name": "updated",
                "active": false,
                "show_in_charts": true,
                "order": 10,
            }),
            &cookie,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = read_json_body(res).await;
    assert_eq!(body["name"], "updated");
    assert_eq!(body["active"], false);
    assert_eq!(body["show_in_charts"], true);
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
    assert!(row.show_in_charts);
    assert_eq!(row.order, Some(10));
    assert_eq!(row.description.as_deref(), Some("original desc"));
}

#[sqlx::test]
async fn update_chain_with_invalid_payload(pool: PgPool) {
    let cookie = create_test_session(&pool).await;
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

    let mut req = post_json_without_cookie(
        "/chains",
        json!({
            "active": true, "name": "x", "type": "integer", "aggregate": "sum",
        }),
    );
    req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
    let res = app.ready().await.unwrap().call(req).await.unwrap();
    let id = read_json_body(res).await["id"].as_i64().unwrap();

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
    let cookie = create_test_session(&pool).await;
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

    let mut req = post_json_without_cookie(
        "/chains",
        json!({
            "active": true, "name": "x", "type": "integer", "aggregate": "sum",
        }),
    );
    req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
    let res = app.ready().await.unwrap().call(req).await.unwrap();
    let id = read_json_body(res).await["id"].as_i64().unwrap();

    let res = app
        .ready()
        .await
        .unwrap()
        .call(delete_with_cookie(&format!("/chains/{id}"), &cookie))
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
    let cookie = create_test_session(&pool).await;
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

    let mut req = post_json_without_cookie(
        "/chains",
        json!({
            "active": false,
            "name": "inactive habit",
            "type": "boolean",
            "aggregate": "avg",
            "description": "hidden",
            "order": 3,
            "show_in_charts": true,
        }),
    );
    req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
    let res = app.ready().await.unwrap().call(req).await.unwrap();
    let created = read_json_body(res).await;
    let id = created["id"].as_i64().unwrap();

    let res = app
        .ready()
        .await
        .unwrap()
        .call(get_with_cookie(&format!("/chains/{id}"), &cookie))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let response_body = read_json_body(res).await;
    assert_eq!(response_body, created);
    assert_eq!(response_body["show_in_charts"], true);
}

#[sqlx::test]
async fn show_chains(pool: PgPool) {
    let cookie = create_test_session(&pool).await;
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();

    let items = vec![
        json!({"active": true, "name": "a", "type": "integer", "aggregate": "sum"}),
        json!({"active": true, "name": "b", "type": "float", "aggregate": "avg", "show_in_charts": true}),
        json!({"active": false, "name": "c", "type": "boolean", "aggregate": "sum"}),
    ];

    for payload in &items {
        let mut req = post_json_without_cookie("/chains", payload.clone());
        req.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
        let res = app.ready().await.unwrap().call(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    let res = app
        .ready()
        .await
        .unwrap()
        .call(get_with_cookie("/chains", &cookie))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let list = read_json_body(res).await;
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0]["name"], "c");
    assert_eq!(list[0]["show_in_charts"], false);
    assert_eq!(list[1]["name"], "b");
    assert_eq!(list[1]["show_in_charts"], true);
    assert_eq!(list[2]["name"], "a");
    assert_eq!(list[2]["show_in_charts"], false);
}

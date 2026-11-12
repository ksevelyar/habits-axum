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
async fn user_creates_chain(pool: PgPool) {
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

    for (name, chain_type, aggregate) in [
        ("weight", "float", "avg"),
        ("meditation", "boolean", "sum"),
        ("reading", "time", "sum"),
    ] {
        let mut request = post_json_without_cookie(
            "/chains",
            json!({"active":true,"name":name,"type":chain_type,"aggregate":aggregate}),
        );
        request.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
        let response = app.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let chain = read_json_body(response).await;
        assert_eq!(chain["type"], chain_type);
        assert_eq!(chain["name"], name);
        assert_eq!(chain["aggregate"], aggregate);
    }
}

#[sqlx::test]
async fn user_cannot_create_chain_with_invalid_input(pool: PgPool) {
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
async fn user_enables_chain_in_charts(pool: PgPool) {
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
async fn user_enables_chain_in_charts_with_invalid_payload(pool: PgPool) {
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
async fn user_deletes_chain(pool: PgPool) {
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
async fn user_reads_chain(pool: PgPool) {
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
async fn user_reads_chains(pool: PgPool) {
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

#[sqlx::test]
async fn user_cannot_read_another_users_chain(pool: PgPool) {
    let owner_cookie = create_user_and_session(&pool, "owner@example.com", "x", "Europe/London", "owner").await;
    let another_cookie = create_user_and_session(&pool, "another@example.com", "x", "Europe/London", "another").await;
    let mut application = build_app(pool, None, Arc::new(SystemClock)).into_service();
    let mut request = post_json_without_cookie(
        "/chains",
        json!({"active":true,"name":"private","type":"integer","aggregate":"sum"}),
    );
    request
        .headers_mut()
        .insert(header::COOKIE, owner_cookie.parse().unwrap());
    let chain_id = read_json_body(application.ready().await.unwrap().call(request).await.unwrap()).await["id"]
        .as_i64()
        .unwrap();
    let response = application
        .ready()
        .await
        .unwrap()
        .call(get_with_cookie(&format!("/chains/{chain_id}"), &another_cookie))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(read_json_body(response).await, json!({"error":"chain not found"}));
}

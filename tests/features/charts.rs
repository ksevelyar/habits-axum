use axum::http::{StatusCode, header};
use jiff::civil::Date;
use jiff_sqlx::ToSqlx;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tower::{Service, ServiceExt};

use super::support::{
    create_session, create_user, get_request, get_with_cookie, post_json_without_cookie, read_json_body,
};
use habits_axum::build_app;
use habits_axum::clock::SystemClock;

async fn create_chain(pool: &PgPool, cookie: &str, name: &str, active: bool, show_in_charts: bool) -> Value {
    let mut app = build_app(pool.clone(), None, Arc::new(SystemClock)).into_service();
    let mut request = post_json_without_cookie(
        "/chains",
        json!({
            "active": active,
            "name": name,
            "type": "integer",
            "aggregate": "sum",
            "show_in_charts": show_in_charts,
        }),
    );
    request.headers_mut().insert(header::COOKIE, cookie.parse().unwrap());
    let response = app.ready().await.unwrap().call(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    read_json_body(response).await
}

#[sqlx::test]
async fn require_authentication_for_charts(pool: PgPool) {
    let mut app = build_app(pool, None, Arc::new(SystemClock)).into_service();
    let response = app
        .ready()
        .await
        .unwrap()
        .call(get_request("/charts?start_date=2026-02-09&end_date=2026-03-09"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn return_only_active_enabled_chains_for_the_authenticated_user(pool: PgPool) {
    create_user(&pool, "chart-owner@example.com").await;
    create_user(&pool, "chart-other-owner@example.com").await;
    let owner_cookie = create_session(&pool, "chart-owner@example.com").await;
    let other_owner_cookie = create_session(&pool, "chart-other-owner@example.com").await;

    let active_chart_chain = create_chain(&pool, &owner_cookie, "Pull-ups", true, true).await;
    create_chain(&pool, &owner_cookie, "Chart-disabled", true, false).await;
    create_chain(&pool, &owner_cookie, "Inactive-chart", false, true).await;
    create_chain(&pool, &other_owner_cookie, "Other-owner-chart", true, true).await;

    let mut app = build_app(pool, None, Arc::new(SystemClock)).into_service();
    let response = app
        .ready()
        .await
        .unwrap()
        .call(get_with_cookie(
            "/charts?start_date=2026-02-09&end_date=2026-03-09",
            &owner_cookie,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let response_body = read_json_body(response).await;
    let response_chains = response_body["chains"].as_array().unwrap();
    assert_eq!(response_chains.len(), 1);
    assert_eq!(response_chains[0]["id"], active_chart_chain["id"]);
    assert_eq!(response_chains[0]["name"], "Pull-ups");
    assert_eq!(response_chains[0]["type"], "integer");
    assert_eq!(response_chains[0]["points"], json!([]));
}

#[sqlx::test]
async fn exclude_metrics_at_the_exclusive_end_date(pool: PgPool) {
    create_user(&pool, "chart-history@example.com").await;
    let cookie = create_session(&pool, "chart-history@example.com").await;
    let chart_chain = create_chain(&pool, &cookie, "Pull-ups", true, true).await;
    let chain_id = chart_chain["id"].as_i64().unwrap();
    let metric_dates_and_values = [
        (Date::new(2026, 2, 8).unwrap(), 10),
        (Date::new(2026, 2, 9).unwrap(), 20),
        (Date::new(2026, 2, 11).unwrap(), 30),
        (Date::new(2026, 2, 13).unwrap(), 40),
        (Date::new(2026, 2, 14).unwrap(), 50),
    ];

    for (metric_date, integer_value) in metric_dates_and_values {
        sqlx::query("INSERT INTO metrics (chain_id, date, value_integer) VALUES ($1, $2, $3)")
            .bind(chain_id)
            .bind(metric_date.to_sqlx())
            .bind(integer_value)
            .execute(&pool)
            .await
            .unwrap();
    }

    let mut app = build_app(pool, None, Arc::new(SystemClock)).into_service();
    let response = app
        .ready()
        .await
        .unwrap()
        .call(get_with_cookie(
            "/charts?start_date=2026-02-09&end_date=2026-02-13",
            &cookie,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let response_body = read_json_body(response).await;
    let response_points = response_body["chains"][0]["points"].as_array().unwrap();
    assert_eq!(response_points.len(), 2);
    assert_eq!(response_points[0]["date"], "2026-02-09");
    assert_eq!(response_points[0]["value"].as_f64().unwrap(), 20.0);
    assert_eq!(response_points[1]["date"], "2026-02-11");
    assert_eq!(response_points[1]["value"].as_f64().unwrap(), 30.0);
}

#[sqlx::test]
async fn reject_empty_or_reversed_chart_ranges(pool: PgPool) {
    create_user(&pool, "chart-invalid-range@example.com").await;
    let cookie = create_session(&pool, "chart-invalid-range@example.com").await;
    let mut app = build_app(pool, None, Arc::new(SystemClock)).into_service();

    for requested_range in [
        "/charts?start_date=2026-03-01&end_date=2026-03-01",
        "/charts?start_date=2026-04-01&end_date=2026-03-01",
    ] {
        let response = app
            .ready()
            .await
            .unwrap()
            .call(get_with_cookie(requested_range, &cookie))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            read_json_body(response).await,
            json!({"error": "start_date must be before end_date"})
        );
    }
}

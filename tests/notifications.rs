use axum::http;
use futures_util::StreamExt;
use serde_json::Value;
use sqlx::PgPool;
use std::net::{Ipv4Addr, SocketAddr};
use tokio::net::TcpStream;
use tokio::time::{Duration, timeout};
use tokio_tungstenite::{client_async, tungstenite};

use habits_axum::{app, authentication, users};

async fn insert_user(pool: &PgPool) {
    let hash = crate::users::hash("pass").unwrap();
    sqlx::query("INSERT INTO users (email, password_hash, handle) VALUES ($1, $2, $3)")
        .bind("test@test.com")
        .bind(&hash)
        .bind("trinity")
        .execute(pool)
        .await
        .unwrap();
}

#[sqlx::test]
async fn authenticate_with_valid_jwt_via_cookie(pool: PgPool) {
    insert_user(&pool).await;

    let jwt = authentication::encode_jwt("test@test.com".to_string()).unwrap();

    let listener = tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(axum::serve(listener, app(pool.clone())).into_future());

    let uri: http::Uri = format!("ws://{addr}/websocket/notifications")
        .parse()
        .unwrap();
    let builder =
        tungstenite::ClientRequestBuilder::new(uri).with_header("Cookie", format!("jwt={jwt}"));

    let tcp = TcpStream::connect(addr).await.unwrap();
    let (mut socket, _) = client_async(builder, tcp).await.unwrap();

    let msg = match socket.next().await.unwrap().unwrap() {
        tungstenite::Message::Text(msg) => msg,
        other => panic!("expected text, got {other:?}"),
    };

    let response: Value = serde_json::from_str(&msg).unwrap();
    assert_eq!(response["event"], "UserAuthenticated");
    assert_eq!(response["user"]["email"], "test@test.com");
    assert!(response["user"]["id"].as_i64().is_some());
}

#[sqlx::test]
async fn authenticate_with_valid_jwt_via_bearer(pool: PgPool) {
    insert_user(&pool).await;

    let jwt = authentication::encode_jwt("test@test.com".to_string()).unwrap();

    let listener = tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(axum::serve(listener, app(pool.clone())).into_future());

    let uri: http::Uri = format!("ws://{addr}/websocket/notifications")
        .parse()
        .unwrap();
    let builder = tungstenite::ClientRequestBuilder::new(uri)
        .with_header("Authorization", format!("Bearer {jwt}"));

    let tcp = TcpStream::connect(addr).await.unwrap();
    let (mut socket, _) = client_async(builder, tcp).await.unwrap();

    let msg = match socket.next().await.unwrap().unwrap() {
        tungstenite::Message::Text(msg) => msg,
        other => panic!("expected text, got {other:?}"),
    };

    let response: Value = serde_json::from_str(&msg).unwrap();
    assert_eq!(response["event"], "UserAuthenticated");
    assert_eq!(response["user"]["email"], "test@test.com");
    assert!(response["user"]["id"].as_i64().is_some());
}

#[sqlx::test]
async fn authenticate_with_invalid_jwt(pool: PgPool) {
    let listener = tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(axum::serve(listener, app(pool.clone())).into_future());

    let uri: http::Uri = format!("ws://{addr}/websocket/notifications")
        .parse()
        .unwrap();
    let builder =
        tungstenite::ClientRequestBuilder::new(uri).with_header("Cookie", "jwt=invalid.token.here");

    let tcp = TcpStream::connect(addr).await.unwrap();
    let result = client_async(builder, tcp).await;
    assert!(result.is_err());
}

#[sqlx::test]
async fn send_three_cron_reminders(pool: PgPool) {
    insert_user(&pool).await;

    let jwt = authentication::encode_jwt("test@test.com".to_string()).unwrap();

    let (user_id,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email = $1")
        .bind("test@test.com")
        .fetch_one(&pool)
        .await
        .unwrap();

    let crons = ["0/3 * * * * *", "1/3 * * * * *", "2/3 * * * * *"];
    for (i, cron_expr) in crons.iter().enumerate() {
        sqlx::query("INSERT INTO tasks (user_id, name, cron, active) VALUES ($1, $2, $3, true)")
            .bind(user_id)
            .bind(format!("Task {}", i + 1))
            .bind(cron_expr)
            .execute(&pool)
            .await
            .unwrap();
    }

    let listener = tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(axum::serve(listener, app(pool.clone())).into_future());

    let uri: http::Uri = format!("ws://{addr}/websocket/notifications")
        .parse()
        .unwrap();
    let builder =
        tungstenite::ClientRequestBuilder::new(uri).with_header("Cookie", format!("jwt={jwt}"));

    let tcp = TcpStream::connect(addr).await.unwrap();
    let (mut socket, _) = client_async(builder, tcp).await.unwrap();

    let msg = match socket.next().await.unwrap().unwrap() {
        tungstenite::Message::Text(m) => m,
        other => panic!("expected text, got {other:?}"),
    };
    let response: Value = serde_json::from_str(&msg).unwrap();
    assert_eq!(response["event"], "UserAuthenticated");

    let mut reminders = Vec::new();
    while reminders.len() < 3 {
        match timeout(Duration::from_secs(10), socket.next()).await {
            Ok(Some(Ok(tungstenite::Message::Text(text)))) => {
                let event: Value = serde_json::from_str(&text).unwrap();
                if event["event"] == "TaskReminder" {
                    assert!(event["task_id"].as_i64().is_some());
                    assert!(event["task_name"].as_str().is_some());
                    reminders.push(event);
                }
            }
            Ok(Some(Ok(tungstenite::Message::Close(_)))) => break,
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => panic!("ws error: {e}"),
            Ok(None) => break,
            Err(_) => panic!(
                "timeout waiting for task reminders, got {}/3",
                reminders.len()
            ),
        }
    }

    assert_eq!(
        reminders.len(),
        3,
        "got {} reminders: {reminders:?}",
        reminders.len()
    );

    let task_ids: Vec<i64> = reminders
        .iter()
        .map(|r| r["task_id"].as_i64().unwrap())
        .collect();
    let mut unique_ids = task_ids.clone();
    unique_ids.sort();
    unique_ids.dedup();
    assert_eq!(
        unique_ids.len(),
        3,
        "should cover 3 distinct tasks, got ids {task_ids:?}"
    );

    let names: Vec<String> = reminders
        .iter()
        .map(|r| r["task_name"].as_str().unwrap().to_string())
        .collect();
    for i in 1..=3 {
        assert!(
            names.contains(&format!("Task {i}")),
            "missing reminder for Task {i}, got names {names:?}"
        );
    }
}

use axum::http;
use futures_util::StreamExt;
use jiff::Timestamp;
use serde_json::Value;
use sqlx::PgPool;
use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::time::{Duration, timeout};
use tokio_tungstenite::{client_async, tungstenite};

use habits_axum::{
    authentication, build_app,
    clock::{Clock, SystemClock},
};

#[derive(Debug)]
struct FixedClock {
    current_time: Timestamp,
}

impl Clock for FixedClock {
    fn read_current_time(&self) -> Timestamp {
        self.current_time
    }
}

async fn insert_user_with_timezone(pool: &PgPool, timezone: &str) {
    let hash = habits_axum::authentication::hash("pass").unwrap();
    sqlx::query("INSERT INTO users (email, password_hash, handle, timezone) VALUES ($1, $2, $3, $4)")
        .bind("test@test.com")
        .bind(&hash)
        .bind("trinity")
        .bind(timezone)
        .execute(pool)
        .await
        .unwrap();
}

async fn insert_user(pool: &PgPool) {
    insert_user_with_timezone(pool, "UTC").await;
}

#[sqlx::test]
async fn authenticate_with_valid_jwt_via_cookie(pool: PgPool) {
    insert_user(&pool).await;

    let jwt = authentication::encode_jwt("test@test.com".to_string()).unwrap();

    let listener = tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(axum::serve(listener, build_app(pool.clone(), None, Arc::new(SystemClock))).into_future());

    let uri: http::Uri = format!("ws://{addr}/websocket/notifications").parse().unwrap();
    let builder = tungstenite::ClientRequestBuilder::new(uri).with_header("Cookie", format!("jwt={jwt}"));

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
    tokio::spawn(axum::serve(listener, build_app(pool.clone(), None, Arc::new(SystemClock))).into_future());

    let uri: http::Uri = format!("ws://{addr}/websocket/notifications").parse().unwrap();
    let builder = tungstenite::ClientRequestBuilder::new(uri).with_header("Authorization", format!("Bearer {jwt}"));

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
async fn sends_last_notification_on_connect(pool: PgPool) {
    let fixed_time: Timestamp = "2025-01-08T17:00:00Z".parse().unwrap();
    insert_user_with_timezone(&pool, "Europe/Moscow").await;

    let (user_id,): (i64,) = sqlx::query_as("SELECT id FROM users WHERE email = $1")
        .bind("test@test.com")
        .fetch_one(&pool)
        .await
        .unwrap();

    let evening_cron = "0 0 16 * * 1-5";
    let morning_cron = "0 0 9 * * 3";

    sqlx::query("INSERT INTO tasks (user_id, name, cron, active) VALUES ($1, $2, $3, true)")
        .bind(user_id)
        .bind("Evening Task")
        .bind(evening_cron)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO tasks (user_id, name, cron, active) VALUES ($1, $2, $3, true)")
        .bind(user_id)
        .bind("Morning Task")
        .bind(morning_cron)
        .execute(&pool)
        .await
        .unwrap();

    let jwt = authentication::encode_jwt("test@test.com".to_string()).unwrap();

    let listener = tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(
        axum::serve(
            listener,
            build_app(
                pool.clone(),
                None,
                Arc::new(FixedClock {
                    current_time: fixed_time,
                }),
            ),
        )
        .into_future(),
    );

    let uri: http::Uri = format!("ws://{addr}/websocket/notifications").parse().unwrap();
    let builder = tungstenite::ClientRequestBuilder::new(uri).with_header("Cookie", format!("jwt={jwt}"));

    let tcp = TcpStream::connect(addr).await.unwrap();
    let (mut socket, _) = client_async(builder, tcp).await.unwrap();

    let msg = match socket.next().await.unwrap().unwrap() {
        tungstenite::Message::Text(msg) => msg,
        other => panic!("expected text, got {other:?}"),
    };
    let authenticated: Value = serde_json::from_str(&msg).unwrap();
    assert_eq!(authenticated["event"], "UserAuthenticated");

    let msg = match socket.next().await.unwrap().unwrap() {
        tungstenite::Message::Text(msg) => msg,
        other => panic!("expected text, got {other:?}"),
    };
    let replay: Value = serde_json::from_str(&msg).unwrap();
    assert_eq!(replay["event"], "TaskReminder");
    assert_eq!(replay["task_name"], "Evening Task");
    assert_eq!(replay["scheduled_time"], "16:00");
}

#[sqlx::test]
async fn authenticate_with_invalid_jwt(pool: PgPool) {
    let listener = tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(axum::serve(listener, build_app(pool.clone(), None, Arc::new(SystemClock))).into_future());

    let uri: http::Uri = format!("ws://{addr}/websocket/notifications").parse().unwrap();
    let builder = tungstenite::ClientRequestBuilder::new(uri).with_header("Cookie", "jwt=invalid.token.here");

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
    for (task_index, cron_expression) in crons.iter().enumerate() {
        sqlx::query("INSERT INTO tasks (user_id, name, cron, active) VALUES ($1, $2, $3, true)")
            .bind(user_id)
            .bind(format!("Task {}", task_index + 1))
            .bind(cron_expression)
            .execute(&pool)
            .await
            .unwrap();
    }

    let listener = tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(axum::serve(listener, build_app(pool.clone(), None, Arc::new(SystemClock))).into_future());

    let uri: http::Uri = format!("ws://{addr}/websocket/notifications").parse().unwrap();
    let builder = tungstenite::ClientRequestBuilder::new(uri).with_header("Cookie", format!("jwt={jwt}"));

    let tcp = TcpStream::connect(addr).await.unwrap();
    let (mut socket, _) = client_async(builder, tcp).await.unwrap();

    let msg = match socket.next().await.unwrap().unwrap() {
        tungstenite::Message::Text(m) => m,
        other => panic!("expected text, got {other:?}"),
    };
    let response: Value = serde_json::from_str(&msg).unwrap();
    assert_eq!(response["event"], "UserAuthenticated");

    let mut reminders = Vec::new();
    let mut reminder_task_ids = HashSet::new();
    while reminder_task_ids.len() < 3 {
        match timeout(Duration::from_secs(10), socket.next()).await {
            Ok(Some(Ok(tungstenite::Message::Text(text)))) => {
                let event: Value = serde_json::from_str(&text).unwrap();
                if event["event"] == "TaskReminder" {
                    let task_id = event["task_id"].as_i64().unwrap();
                    assert!(event["task_name"].as_str().is_some());
                    reminder_task_ids.insert(task_id);
                    reminders.push(event);
                }
            }
            Ok(Some(Ok(tungstenite::Message::Close(_)))) => break,
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => panic!("ws error: {e}"),
            Ok(None) => break,
            Err(_) => panic!(
                "timeout waiting for all task reminders, got {}/3 distinct task ids",
                reminder_task_ids.len()
            ),
        }
    }

    assert_eq!(
        reminder_task_ids.len(),
        3,
        "should cover 3 distinct tasks, got events {reminders:?}"
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

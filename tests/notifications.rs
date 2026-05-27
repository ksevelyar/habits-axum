use axum::http;
use futures_util::StreamExt;
use serde_json::Value;
use sqlx::PgPool;
use std::net::{Ipv4Addr, SocketAddr};
use tokio::net::TcpStream;
use tokio_tungstenite::{client_async, tungstenite};

use habits_axum::{app, users};

#[sqlx::test]
async fn authenticate_with_valid_jwt_via_cookie(pool: PgPool) {
    let hash = bcrypt::hash("pass", bcrypt::DEFAULT_COST).unwrap();
    sqlx::query("INSERT INTO users (email, password_hash) VALUES ($1, $2)")
        .bind("test@test.com")
        .bind(&hash)
        .execute(&pool)
        .await
        .unwrap();

    let jwt = users::encode_jwt("test@test.com".to_string()).unwrap();

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
    let hash = bcrypt::hash("pass", bcrypt::DEFAULT_COST).unwrap();
    sqlx::query("INSERT INTO users (email, password_hash) VALUES ($1, $2)")
        .bind("test@test.com")
        .bind(&hash)
        .execute(&pool)
        .await
        .unwrap();

    let jwt = users::encode_jwt("test@test.com".to_string()).unwrap();

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

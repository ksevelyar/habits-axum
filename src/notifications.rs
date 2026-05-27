use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;
use serde_json::json;
use sqlx::PgPool;

use crate::error::AppError;
use crate::users;

pub async fn connect(
    ws: WebSocketUpgrade,
    cookie_jar: CookieJar,
    headers: HeaderMap,
    State(pool): State<PgPool>,
) -> Response {
    let token = cookie_jar
        .get("jwt")
        .map(|c| c.value().to_string())
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .map(|v| v.to_string())
        });

    let user = match token {
        Some(t) => match users::authenticate_token(&pool, &t).await {
            Ok(u) => u,
            Err(_) => return AppError::Unauthorized.into_response(),
        },
        None => return AppError::Unauthorized.into_response(),
    };

    ws.on_upgrade(move |socket| handle_connection(socket, user))
}

async fn handle_connection(mut socket: WebSocket, user: users::User) {
    let ack = json!({"event": "UserAuthenticated", "user": user});
    if socket
        .send(Message::Text(ack.to_string().into()))
        .await
        .is_err()
    {
        return;
    }

    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg
            && socket.send(Message::Text(text)).await.is_err()
        {
            break;
        }
    }
}

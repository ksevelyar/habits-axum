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
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{Duration, Instant};

use crate::AppState;
use crate::authentication::authenticate_request;
use crate::users;
use futures_util::{sink::SinkExt, stream::StreamExt};

pub async fn connect(
    ws: WebSocketUpgrade,
    cookie_jar: CookieJar,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Response {
    let user = match authenticate_request(&state.pool, &cookie_jar, &headers).await {
        Ok(user) => user,
        Err(error) => return error.into_response(),
    };

    ws.on_upgrade(move |socket| handle_connection(socket, user, state))
}

async fn handle_connection(socket: WebSocket, user: users::User, state: Arc<AppState>) {
    let user_channel = crate::notifications::ensure_delivery(state.clone(), &user).await;
    let mut broadcast_rx = user_channel.subscribe();
    let (mut sender, mut receiver) = socket.split();

    tracing::info!(
        user_id = &user.id,
        receiver_count = &user_channel.receiver_count(),
        "websocket connected"
    );

    let user_authenticated = json!({"event": "UserAuthenticated", "user": user});
    if let Err(e) = sender.send(Message::Text(user_authenticated.to_string().into())).await {
        tracing::warn!(user_id = user.id, error = %e, "failed to send UserAuthenticated");
        return;
    }

    let mut ping_interval = tokio::time::interval(Duration::from_secs(30));
    ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut pending_ping: Option<Instant> = None;

    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Pong(_))) => {
                        if let Some(sent) = pending_ping.take() {
                            tracing::debug!(
                                user_id = user.id,
                                elapsed_secs = sent.elapsed().as_secs_f64(),
                                "pong received"
                            );
                        }
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    _ => continue,
                }
            }

            msg = broadcast_rx.recv() => {
                match msg {
                    Ok(msg) => {
                        if sender.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(user_id = user.id, missed = n, "broadcast lag");
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }

            _ = ping_interval.tick() => {
                if pending_ping.replace(Instant::now()).is_some() {
                    tracing::warn!(
                        user_id = user.id,
                        "pong not received within 30s interval, closing connection"
                    );
                    break;
                }
                if sender.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
                tracing::debug!(user_id = user.id, "ping sent");
            }
        }
    }

    tracing::info!(
        user_id = user.id,
        receiver_count = user_channel.receiver_count(),
        "websocket disconnected"
    );
}

use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;
use chrono::{DateTime, Utc};
use cron::Schedule;
use serde_json::json;
use sqlx::PgPool;
use std::str::FromStr;

use crate::error::AppError;
use crate::tasks;
use crate::users;

use crate::{AppState, UserChannel};
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{Duration, Instant};

pub async fn connect(
    ws: WebSocketUpgrade,
    cookie_jar: CookieJar,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Response {
    let token = cookie_jar.get("jwt").map(|c| c.value()).or_else(|| {
        headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
    });

    let Some(token) = token else {
        return AppError::Unauthorized.into_response();
    };

    let Ok(user) = users::authenticate_token(&state.pool, token).await else {
        return AppError::Unauthorized.into_response();
    };

    ws.on_upgrade(move |socket| handle_connection(socket, user, state))
}

async fn handle_connection(socket: WebSocket, user: users::User, state: Arc<AppState>) {
    let user_channel = get_or_create_user_channel(state.clone(), &user).await;
    let mut broadcast_rx = user_channel.subscribe();
    let (mut sender, mut receiver) = socket.split();

    tracing::info!(
        user_id = &user.id,
        receiver_count = &user_channel.receiver_count(),
        "websocket connected"
    );

    let user_authenticated = json!({"event": "UserAuthenticated", "user": user});
    if let Err(e) = sender
        .send(Message::Text(user_authenticated.to_string().into()))
        .await
    {
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
                            tracing::info!(
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
                tracing::info!(user_id = user.id, "ping sent");
            }
        }
    }

    tracing::info!(
        user_id = user.id,
        receiver_count = user_channel.receiver_count(),
        "websocket disconnected"
    );
}

async fn get_or_create_user_channel(
    state: Arc<AppState>,
    user: &users::User,
) -> broadcast::Sender<String> {
    {
        let fast_path = state.channels.read().await;
        if let Some(entry) = fast_path.get(&user.id)
            && !entry.scheduler.is_finished()
        {
            return entry.broadcast.clone();
        }
    }

    let mut slow_path = state.channels.write().await;
    if let Some(entry) = slow_path.get(&user.id)
        && !entry.scheduler.is_finished()
    {
        return entry.broadcast.clone();
    }

    let (broadcast_tx, _) = broadcast::channel::<String>(100);
    let pool = state.pool.clone();
    let scheduler_state = state.clone();
    let scheduler = tokio::spawn(user_scheduler(
        user.clone(),
        broadcast_tx.clone(),
        pool,
        scheduler_state,
    ));
    slow_path.insert(
        user.id,
        UserChannel {
            broadcast: broadcast_tx.clone(),
            scheduler,
        },
    );

    broadcast_tx
}

fn eval_next_run(
    tasks: &[tasks::Task],
    tz: chrono_tz::Tz,
) -> Option<(&tasks::Task, DateTime<Utc>)> {
    tasks
        .iter()
        .filter(|task| task.active)
        .filter_map(|task| {
            let schedule = Schedule::from_str(&task.cron).ok()?;
            let next_run = schedule.upcoming(tz).next()?;
            Some((task, next_run.with_timezone(&Utc)))
        })
        .min_by_key(|(_, next_run)| *next_run)
}

async fn user_scheduler(
    user: users::User,
    broadcast_tx: broadcast::Sender<String>,
    pool: PgPool,
    state: Arc<AppState>,
) {
    let mut empty_ticks = 0;

    loop {
        let now = Utc::now();
        let tasks = match tasks::list_by_user_id(&pool, user.id).await {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::error!("failed to load tasks: {e}");
                break;
            }
        };

        let tz: chrono_tz::Tz = user.timezone.parse().unwrap();
        let (task, next_run_at) = match eval_next_run(&tasks, tz) {
            Some(v) => v,
            None => break,
        };

        let scheduled_time = next_run_at.with_timezone(&tz).format("%H:%M").to_string();
        tracing::info!(
            task_id = task.id,
            task_name = task.name,
            scheduled_time = scheduled_time,
            connected_clients = broadcast_tx.receiver_count(),
        );

        let sleep_duration = next_run_at - now;
        tokio::time::sleep(sleep_duration.to_std().unwrap()).await;

        let msg = json!({
            "event": "TaskReminder",
            "task_id": task.id,
            "task_name": task.name,
            "scheduled_time": scheduled_time
        });

        if broadcast_tx.receiver_count() == 0 {
            tracing::warn!(
                user_id = user.id,
                task_id = task.id,
                "no connected clients, notification dropped"
            );
            empty_ticks += 1;
            if empty_ticks >= 3 {
                tracing::warn!("no clients for 3 ticks, shutting down scheduler");
                break;
            }
        } else {
            empty_ticks = 0;
        }

        match broadcast_tx.send(msg.to_string()) {
            Ok(count) => tracing::info!(count, "notification sent"),
            Err(e) => tracing::warn!(error = %e, "failed to send notification"),
        }
    }

    let mut channels = state.channels.write().await;
    channels.remove(&user.id);
}

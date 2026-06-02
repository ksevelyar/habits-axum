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

use crate::{AppState, UserEntry};
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

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
    let tx = get_or_create_user_channel(state.clone(), &user).await;
    let mut rx = tx.subscribe();

    let (mut sender, mut receiver) = socket.split();

    let ack = json!({"event": "UserAuthenticated", "user": user});
    if let Err(e) = sender.send(Message::Text(ack.to_string().into())).await {
        tracing::warn!(user_id = user.id, error = %e, "failed to send UserAuthenticated");
        return;
    }

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Err(e) = sender.send(Message::Text(msg.into())).await {
                tracing::warn!(error = %e, "failed to send notification to client");
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            match result {
                Ok(Message::Close(_)) | Err(_) => break,
                Ok(_) => {}
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    };

    tracing::info!(user_id = user.id, "disconnected");
}

async fn get_or_create_user_channel(
    state: Arc<AppState>,
    user: &users::User,
) -> broadcast::Sender<String> {
    if let Some(tx) = {
        let fast_path = state.users.read().await;
        fast_path.get(&user.id).map(|entry| entry.tx.clone())
    } {
        return tx;
    }

    let mut slow_path = state.users.write().await;
    if let Some(tx) = slow_path.get(&user.id).map(|entry| entry.tx.clone()) {
        return tx;
    }

    let (tx, _) = broadcast::channel::<String>(100);
    tokio::spawn(user_scheduler(user.clone(), tx.clone(), state.pool.clone()));
    slow_path.insert(user.id, UserEntry { tx: tx.clone() });

    tx
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

async fn user_scheduler(user: users::User, tx: broadcast::Sender<String>, pool: PgPool) {
    loop {
        let now = Utc::now();
        let tasks = match tasks::list_by_user_id(&pool, user.id).await {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::error!("failed to load tasks: {e}");
                return;
            }
        };

        let tz: chrono_tz::Tz = user.timezone.parse().unwrap();
        let (task, next_run_at) = match eval_next_run(&tasks, tz) {
            Some(v) => v,
            None => break,
        };

        let scheduled_at = next_run_at.with_timezone(&tz);
        tracing::info!(
            task_id = task.id, task_name = task.name,
            scheduled_at = %scheduled_at,
            connected_clients = tx.receiver_count(),
        );

        let sleep_duration = next_run_at - now;
        tokio::time::sleep(sleep_duration.to_std().unwrap()).await;

        let msg = json!({
            "event": "TaskReminder",
            "task_id": task.id,
            "task_name": task.name,
            "scheduled_at": scheduled_at
        });

        // NOTE: maybe drop scheduler if no clients connected more than 3 ticks
        if tx.receiver_count() == 0 {
            tracing::warn!(
                user_id = user.id,
                task_id = task.id,
                "no connected clients, notification dropped"
            );
        }
        let _ = tx.send(msg.to_string());
    }
}

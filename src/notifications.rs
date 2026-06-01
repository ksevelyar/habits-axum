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
        Some(t) => match users::authenticate_token(&state.pool, &t).await {
            Ok(u) => u,
            Err(_) => return AppError::Unauthorized.into_response(),
        },
        None => return AppError::Unauthorized.into_response(),
    };

    ws.on_upgrade(move |socket| handle_connection(socket, user, state))
}

async fn handle_connection(mut socket: WebSocket, user: users::User, state: Arc<AppState>) {
    let tx = get_or_create_user_channel(state.clone(), &user).await;

    let mut rx = tx.subscribe();

    let ack = json!({"event": "UserAuthenticated", "user": user});
    if socket
        .send(Message::Text(ack.to_string().into()))
        .await
        .is_err()
    {
        return;
    }
    let (mut sender, mut receiver) = socket.split();

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
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

    println!("Disconnected user {}", user.id);
}

async fn get_or_create_user_channel(
    state: Arc<AppState>,
    user: &users::User,
) -> broadcast::Sender<String> {
    {
        let users = state.users.read().await;

        if let Some(entry) = users.get(&user.id) {
            return entry.tx.clone();
        }
    }

    let mut users = state.users.write().await;

    if let Some(entry) = users.get(&user.id) {
        return entry.tx.clone();
    }

    let (tx, _) = broadcast::channel::<String>(100);

    tokio::spawn(user_scheduler(user.clone(), tx.clone(), state.pool.clone()));

    users.insert(user.id, UserEntry { tx: tx.clone() });

    tx
}

fn eval_next_run(
    tasks: &[tasks::Task],
    tz: chrono_tz::Tz,
) -> Option<(&tasks::Task, DateTime<Utc>)> {
    let now_local = Utc::now().with_timezone(&tz);

    tasks
        .iter()
        .filter(|task| task.active)
        .filter_map(|task| {
            let schedule = Schedule::from_str(&task.cron).ok()?;

            let next_local = schedule.upcoming(tz).find(|t| *t > now_local)?;

            let next_utc = next_local.with_timezone(&Utc);

            Some((task, next_utc))
        })
        .min_by_key(|(_, t)| *t)
}

async fn user_scheduler(user: users::User, tx: broadcast::Sender<String>, pool: PgPool) {
    let tasks = match tasks::list_by_user_id(&pool, user.id).await {
        Ok(tasks) => tasks,
        Err(e) => {
            tracing::error!("failed to load tasks: {e}");
            return;
        }
    };

    let tz: chrono_tz::Tz = user.timezone.parse().unwrap();

    loop {
        let now = Utc::now();
        let (task, next_run_at) = match eval_next_run(&tasks, tz) {
            Some(v) => v,
            None => break,
        };

        let scheduled_at = next_run_at.with_timezone(&tz);
        tracing::info!(task_id = task.id, task_name = task.name, scheduled_at = %scheduled_at);

        let sleep_duration = next_run_at - now;
        tokio::time::sleep(sleep_duration.to_std().unwrap()).await;

        let msg = json!({
            "event": "TaskReminder",
            "task_id": task.id,
            "task_name": task.name,
            "scheduled_at": scheduled_at
        });

        let _ = tx.send(msg.to_string());
    }
}

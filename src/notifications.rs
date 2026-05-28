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
use std::time::Duration;

use crate::error::AppError;
use crate::tasks;
use crate::users;

struct CronTask {
    id: i64,
    name: String,
    schedule: Schedule,
    next_run: Option<DateTime<Utc>>,
}

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

    ws.on_upgrade(move |socket| handle_connection(socket, user, pool))
}

async fn handle_connection(mut socket: WebSocket, user: users::User, pool: PgPool) {
    let ack = json!({"event": "UserAuthenticated", "user": user});
    if socket
        .send(Message::Text(ack.to_string().into()))
        .await
        .is_err()
    {
        return;
    }

    let rows = match tasks::list_by_user_id(&pool, user.id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("failed to load tasks: {e}");
            return;
        }
    };

    let mut scheduled: Vec<CronTask> = Vec::new();
    for task in &rows {
        if !task.active {
            continue;
        }
        let cron_str = match &task.cron {
            Some(c) if !c.is_empty() => c,
            _ => continue,
        };
        let schedule = match Schedule::from_str(cron_str) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("invalid cron '{}': {e}", cron_str);
                continue;
            }
        };
        scheduled.push(CronTask {
            id: task.id,
            name: task.name.clone().unwrap_or_default(),
            schedule,
            next_run: None,
        });
    }

    for t in &mut scheduled {
        t.next_run = t.schedule.upcoming(Utc).next();
        if let Some(r) = t.next_run {
            tracing::info!(task_id = t.id, task_name = %t.name, next_run_at = %r);
        }
    }

    if scheduled.is_empty() {
        tracing::info!("no scheduled tasks");
    }

    loop {
        let now = Utc::now();

        for t in &mut scheduled {
            if t.next_run.is_none() {
                t.next_run = t.schedule.upcoming(Utc).next();
            }
        }

        let next = scheduled
            .iter()
            .filter_map(|t| t.next_run)
            .filter(|t| *t > now)
            .min()
            .map(|t| t - now)
            .and_then(|d| d.to_std().ok())
            .unwrap_or(Duration::from_secs(60));

        tokio::time::sleep(next).await;

        let now = Utc::now();
        for t in &mut scheduled {
            if t.next_run.is_some_and(|r| r <= now) {
                let msg = json!({
                    "event": "TaskReminder",
                    "task_id": t.id,
                    "task_name": t.name,
                });
                if socket
                    .send(Message::Text(msg.to_string().into()))
                    .await
                    .is_err()
                {
                    return;
                }
                t.next_run = t.schedule.upcoming(Utc).next();
            }
        }
    }
}

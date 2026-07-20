use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::tasks;
use crate::users;
use crate::{AppState, UserChannel};

pub async fn get_or_create_user_channel(state: Arc<AppState>, user: &users::User) -> broadcast::Sender<String> {
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

async fn user_scheduler(
    user: users::User,
    broadcast_tx: broadcast::Sender<String>,
    pool: PgPool,
    state: Arc<AppState>,
) {
    let mut empty_ticks = 0;

    loop {
        let now = Utc::now();
        let (task, next_run_at) = match tasks::eval_next_notification(&pool, &user).await {
            Some(v) => v,
            None => break,
        };

        let tz: chrono_tz::Tz = user.timezone.parse().unwrap();
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

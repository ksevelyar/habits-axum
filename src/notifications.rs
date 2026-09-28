use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use cron::Schedule;
use serde_json::json;
use sqlx::PgPool;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::error::AppError;
use crate::tasks::{self, Task};
use crate::telegram;
use crate::users;
use crate::{AppState, UserChannel};

async fn eval_next_notification(
    pool: &PgPool,
    user: &users::User,
    timezone: Tz,
) -> Result<Option<(Task, DateTime<Utc>)>, AppError> {
    let tasks = tasks::list_by_user_id(pool, user.id).await?;

    Ok(tasks
        .into_iter()
        .filter(|task| task.active)
        .filter_map(|task| {
            let schedule = Schedule::from_str(&task.cron).ok()?;
            let next_run = schedule.upcoming(timezone).next()?;
            Some((task, next_run.with_timezone(&Utc)))
        })
        .min_by_key(|(_, next_run)| *next_run))
}

pub async fn ensure_delivery(state: Arc<AppState>, user: &users::User) -> broadcast::Sender<String> {
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
    let delivery_state = state.clone();
    let scheduler = tokio::spawn(delivery_loop(user.clone(), broadcast_tx.clone(), pool, delivery_state));
    slow_path.insert(
        user.id,
        UserChannel {
            broadcast: broadcast_tx.clone(),
            scheduler,
        },
    );

    broadcast_tx
}

async fn delivery_loop(user: users::User, broadcast_tx: broadcast::Sender<String>, pool: PgPool, state: Arc<AppState>) {
    let timezone: Tz = user.timezone.parse().expect("timezone is validated on insert");
    loop {
        let now = Utc::now();
        let next_notification = match eval_next_notification(&pool, &user, timezone).await {
            Ok(next) => next,
            Err(error) => {
                tracing::error!(%error, "evaluating next notification failed");
                None
            }
        };
        let Some((task, next_run_at)) = next_notification else {
            break;
        };

        let scheduled_time = next_run_at.with_timezone(&timezone).format("%H:%M").to_string();
        tracing::info!(
            task_id = task.id,
            task_name = task.name,
            scheduled_time = scheduled_time,
            connected_clients = broadcast_tx.receiver_count(),
        );

        let sleep_duration = (next_run_at - now).to_std().expect("next run is in the future");
        tokio::time::sleep(sleep_duration).await;

        if let Some(telegram_state) = &state.telegram
            && let Ok(Some(chat_id)) = telegram::find_chat_id(&pool, user.id).await
        {
            let text = telegram::render_reminder(&task);
            if let Err(error) = telegram_state.send_message(chat_id, &text).await {
                tracing::warn!(?error, "failed to send telegram reminder");
            }
        }

        let msg = json!({
            "event": "TaskReminder",
            "task_id": task.id,
            "task_name": task.name,
            "scheduled_time": scheduled_time
        });

        match broadcast_tx.send(msg.to_string()) {
            Ok(count) => tracing::info!(count, "notification sent"),
            Err(error) => tracing::warn!(%error, "failed to send notification"),
        }
    }

    let mut channels = state.channels.write().await;
    channels.remove(&user.id);
}

pub fn spawn_delivery_sweep(state: Arc<AppState>) {
    tokio::spawn(async move {
        let users_with_active_tasks = sqlx::query_as::<_, users::User>(
            r#"
            SELECT DISTINCT u.id, u.email, u.timezone
            FROM users u
            JOIN tasks t ON t.user_id = u.id
            WHERE t.active
            "#,
        )
        .fetch_all(&state.pool)
        .await;

        match users_with_active_tasks {
            Ok(users) => {
                for user in users {
                    ensure_delivery(state.clone(), &user).await;
                }
            }
            Err(error) => tracing::error!("{error}"),
        }
    });
}

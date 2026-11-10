use croner::parser::{CronParser, Seconds};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::error::AppError;
use crate::tasks::{self, Task};
use crate::telegram;
use crate::users;
use crate::{AppState, UserChannel};

fn find_next_timestamp(cron_expression: &str, timezone_name: &str, current_time: Timestamp) -> Option<Timestamp> {
    let timezone = TimeZone::get(timezone_name).ok()?;
    let current_zoned_time = current_time.to_zoned(timezone);
    let cron = CronParser::builder()
        .seconds(Seconds::Optional)
        .sloppy_ranges(true)
        .build()
        .parse(cron_expression)
        .ok()?;

    cron.find_next_occurrence(&current_zoned_time, false)
        .ok()
        .map(|scheduled_time| scheduled_time.timestamp())
}

fn find_previous_timestamp(cron_expression: &str, timezone_name: &str, current_time: Timestamp) -> Option<Timestamp> {
    let timezone = TimeZone::get(timezone_name).ok()?;
    let current_zoned_time = current_time.to_zoned(timezone);
    let cron = CronParser::builder()
        .seconds(Seconds::Optional)
        .sloppy_ranges(true)
        .build()
        .parse(cron_expression)
        .ok()?;

    cron.find_previous_occurrence(&current_zoned_time, false)
        .ok()
        .map(|scheduled_time| scheduled_time.timestamp())
}

async fn evaluate_next_notification(
    pool: &PgPool,
    user: &users::User,
    current_time: Timestamp,
) -> Result<Option<(Task, Timestamp)>, AppError> {
    let tasks = tasks::list_by_user_id(pool, user.id).await?;

    Ok(tasks
        .into_iter()
        .filter(|task| task.active)
        .filter_map(|task| {
            let next_run = find_next_timestamp(&task.cron, &user.timezone, current_time)?;
            Some((task, next_run))
        })
        .min_by_key(|(_, next_run)| *next_run))
}

pub async fn evaluate_last_notification(
    pool: &PgPool,
    user: &users::User,
    current_time: Timestamp,
) -> Result<Option<(Task, Timestamp)>, AppError> {
    let tasks = tasks::list_by_user_id(pool, user.id).await?;

    Ok(tasks
        .into_iter()
        .filter(|task| task.active)
        .filter_map(|task| {
            let last_run = find_previous_timestamp(&task.cron, &user.timezone, current_time)?;
            Some((task, last_run))
        })
        .max_by_key(|(_, last_run)| *last_run))
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
    let timezone = TimeZone::get(&user.timezone).expect("timezone is validated on insert");
    let mut most_recent_scheduled_timestamp = None;
    loop {
        let clock_time = state.clock.read_current_time();
        let current_time =
            most_recent_scheduled_timestamp.map_or(clock_time, |scheduled_time| clock_time.max(scheduled_time));
        let next_notification = match evaluate_next_notification(&pool, &user, current_time).await {
            Ok(next) => next,
            Err(error) => {
                tracing::error!(%error, "evaluating next notification failed");
                None
            }
        };
        let Some((task, next_run_at)) = next_notification else {
            break;
        };

        let scheduled_time = next_run_at.to_zoned(timezone.clone()).strftime("%H:%M").to_string();
        tracing::info!(
            task_id = task.id,
            task_name = task.name,
            scheduled_time = scheduled_time,
            connected_clients = broadcast_tx.receiver_count(),
        );

        let sleep_duration =
            std::time::Duration::try_from(next_run_at - current_time).expect("next run is in the future");
        tokio::time::sleep(sleep_duration).await;

        if let Some(telegram_state) = &state.telegram
            && let Ok(Some(chat_id)) = telegram::find_chat_id(&pool, user.id).await
        {
            let text = telegram::render_reminder(&task);
            if let Err(error) = telegram_state.send_message(chat_id, &text).await {
                tracing::warn!(?error, "failed to send telegram reminder");
            }
        }

        let message = json!({
            "event": "TaskReminder",
            "task_id": task.id,
            "task_name": task.name,
            "scheduled_time": scheduled_time
        });

        match broadcast_tx.send(message.to_string()) {
            Ok(count) => tracing::info!(count, "notification sent"),
            Err(error) => tracing::warn!(%error, "failed to send notification"),
        }

        most_recent_scheduled_timestamp = Some(next_run_at);
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

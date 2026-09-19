mod client;

pub use client::TelegramClient;

use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use uuid::Uuid;

use crate::AppState;
use crate::notifications;
use crate::tasks::Task;
use crate::users;

use client::Message;

pub fn build() -> Option<TelegramClient> {
    TelegramClient::from_env()
}

pub fn build_bot_link(telegram: &TelegramClient, code: &str) -> String {
    format!("https://t.me/{}?start={code}", telegram.bot_username)
}

pub async fn create_link_code(pool: &PgPool, user_id: i64) -> Result<String, sqlx::Error> {
    let code = generate_link_code();
    sqlx::query("UPDATE users SET telegram_code = $1 WHERE id = $2")
        .bind(&code)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(code)
}

pub async fn find_chat_id(pool: &PgPool, user_id: i64) -> Result<Option<i64>, sqlx::Error> {
    let row: Option<(Option<i64>,)> = sqlx::query_as("SELECT telegram_chat_id FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.and_then(|(chat_id,)| chat_id))
}

pub async fn clear_telegram_chat_id(pool: &PgPool, user_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET telegram_chat_id = NULL WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub fn render_reminder(task: &Task) -> String {
    task.name.clone()
}

pub fn spawn_worker(state: Arc<AppState>) {
    let Some(telegram) = state.telegram.clone() else {
        return;
    };
    tokio::spawn(poll_inbox(state, telegram));
}

async fn consume_link_code(pool: &PgPool, code: &str) -> Result<Option<i64>, sqlx::Error> {
    let user_id: Option<i64> =
        sqlx::query_scalar("UPDATE users SET telegram_code = NULL WHERE telegram_code = $1 RETURNING id")
            .bind(code)
            .fetch_optional(pool)
            .await?;
    Ok(user_id)
}

async fn set_telegram_chat_id(pool: &PgPool, user_id: i64, chat_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET telegram_chat_id = $1 WHERE id = $2")
        .bind(chat_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn poll_inbox(state: Arc<AppState>, telegram: TelegramClient) {
    let mut offset: i64 = 0;

    loop {
        let updates = match telegram.request_updates(offset).await {
            Ok(updates) => updates,
            Err(error) => {
                tracing::warn!(%error, "telegram polling failed");
                tokio::time::sleep(StdDuration::from_secs(10)).await;
                continue;
            }
        };
        if updates.is_empty() {
            tokio::time::sleep(StdDuration::from_secs(1)).await;
            continue;
        }
        for update in updates {
            offset = offset.max(update.id + 1);
            handle_inbox(&state, &telegram, &update).await;
        }
    }
}

async fn handle_inbox(state: &Arc<AppState>, telegram: &TelegramClient, message: &Message) {
    let mut parts = message.text.split_whitespace();
    let command = parts.next().unwrap_or_default().split('@').next().unwrap_or_default();
    let payload = parts.next();

    let reply_text = match (command, payload) {
        ("/start" | "/link", Some(code)) => match link_chat(state, code, message.chat_id).await {
            Ok(text) => text,
            Err(error) => {
                tracing::warn!("telegram command failed: {error}");
                error
            }
        },
        _ => "Available commands: /link <code>".to_string(),
    };

    if let Err(error) = telegram.send_message(message.chat_id, &reply_text).await {
        tracing::error!("sending telegram reply failed: {error}");
    }
}

async fn link_chat(state: &Arc<AppState>, code: &str, chat_id: i64) -> Result<String, String> {
    Uuid::parse_str(code)
        .map_err(|_| "This is not a link code. Get one in the app, then send /link <code>.".to_string())?;

    let user_id = consume_link_code(&state.pool, code)
        .await
        .map_err(|error| {
            tracing::error!("{error}");
            "Something went wrong, please try again later.".to_string()
        })?
        .ok_or("Link code invalid or expired. Request a new one in the app.".to_string())?;

    set_telegram_chat_id(&state.pool, user_id, chat_id)
        .await
        .map_err(|error| {
            tracing::error!("{error}");
            "Something went wrong, please try again later.".to_string()
        })?;

    if let Ok(Some(user)) = users::find_by_id(&state.pool, user_id).await {
        notifications::ensure_delivery(state.clone(), &user).await;
    }
    Ok("Linked ✓".to_string())
}

fn generate_link_code() -> String {
    Uuid::new_v4().to_string()
}

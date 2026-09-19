use axum::Router;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use cookie::Cookie;
use reqwest::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::collections::VecDeque;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, timeout};

use habits_axum::build_app;
use habits_axum::telegram::TelegramClient;

const BOT_TOKEN: &str = "test-bot-token";
const CHAT_ID: i64 = 999;

#[derive(Clone)]
struct MockTelegramApi {
    queued_updates: Arc<Mutex<VecDeque<Value>>>,
    sent_messages: Arc<Mutex<Vec<(i64, String)>>>,
}

async fn serve_queued_updates(State(state): State<MockTelegramApi>, _: Path<String>) -> axum::Json<Value> {
    let updates: Vec<Value> = state.queued_updates.lock().unwrap().drain(..).collect();
    axum::Json(json!({"ok": true, "result": updates}))
}

async fn record_sent_messages(
    State(state): State<MockTelegramApi>,
    _: Path<String>,
    axum::Json(body): axum::Json<Value>,
) -> axum::Json<Value> {
    let chat_id = body["chat_id"].as_i64().unwrap();
    let text = body["text"].as_str().unwrap().to_string();
    state.sent_messages.lock().unwrap().push((chat_id, text));
    axum::Json(json!({"ok": true, "result": {"message_id": 1}}))
}

async fn spawn_mock_telegram_api() -> (SocketAddr, MockTelegramApi) {
    let state = MockTelegramApi {
        queued_updates: Arc::new(Mutex::new(VecDeque::new())),
        sent_messages: Arc::new(Mutex::new(Vec::new())),
    };
    let router = Router::new()
        .route("/{token}/getUpdates", get(serve_queued_updates))
        .route("/{token}/sendMessage", post(record_sent_messages))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(axum::serve(listener, router).into_future());

    (addr, state)
}

async fn start_app_with_mock_telegram_api(pool: PgPool, telegram_api_base_url: &str) -> SocketAddr {
    let telegram = TelegramClient::new(
        reqwest::Client::new(),
        BOT_TOKEN.into(),
        "habits_test_bot".into(),
        telegram_api_base_url.into(),
    );
    let listener = tokio::net::TcpListener::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(axum::serve(listener, build_app(pool, Some(telegram))).into_future());
    addr
}

async fn create_user_session(client: &reqwest::Client, base: &str) -> String {
    let create_user = client
        .post(format!("{base}/users"))
        .json(&json!({
            "email": "telegram@test.com",
            "password": "x",
            "timezone": "Europe/London",
            "handle": "tg_user"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_user.status(), StatusCode::CREATED);

    let create_session = client
        .post(format!("{base}/sessions"))
        .json(&json!({"email": "telegram@test.com", "password": "x"}))
        .send()
        .await
        .unwrap();
    assert_eq!(create_session.status(), StatusCode::CREATED);

    let set_cookie = create_session.headers().get("set-cookie").unwrap().to_str().unwrap();
    let cookie = Cookie::parse(set_cookie).unwrap();
    format!("{}={}", cookie.name(), cookie.value())
}

async fn link_chat_via_bot(client: &reqwest::Client, base: &str, mock: &MockTelegramApi) -> String {
    let cookie = create_user_session(client, base).await;

    let status_response = client
        .get(format!("{base}/telegram/link"))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(status_response.status(), StatusCode::OK);
    let body: Value = status_response.json().await.unwrap();
    assert_eq!(body["linked"], false);

    let link_response = client
        .post(format!("{base}/telegram/link"))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(link_response.status(), StatusCode::OK);
    let body: Value = link_response.json().await.unwrap();
    let url = body["url"].as_str().unwrap();
    assert!(url.starts_with("https://t.me/habits_test_bot?start="));
    let code = url.split("?start=").nth(1).unwrap();

    mock.queued_updates.lock().unwrap().push_back(json!({
        "update_id": 1,
        "message": {
            "message_id": 1,
            "chat": {"id": CHAT_ID},
            "text": format!("/link {code}")
        }
    }));

    wait_for_bot_message(mock, |chat_id, _| *chat_id == CHAT_ID).await;
    cookie
}

async fn wait_for_bot_message(mock: &MockTelegramApi, predicate: impl Fn(&i64, &str) -> bool) {
    timeout(Duration::from_secs(10), async {
        loop {
            let messages = mock.sent_messages.lock().unwrap().clone();
            if messages.iter().any(|(chat_id, text)| predicate(chat_id, text)) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("timed out waiting for telegram sendMessage");
}

#[sqlx::test]
async fn linking_chat_via_bot_code_sets_chat_id(pool: PgPool) {
    let (mock_addr, mock) = spawn_mock_telegram_api().await;
    let app_addr = start_app_with_mock_telegram_api(pool.clone(), &format!("http://{mock_addr}")).await;
    let base = format!("http://{app_addr}");
    let client = reqwest::Client::new();

    let cookie = link_chat_via_bot(&client, &base, &mock).await;

    wait_for_bot_message(&mock, |chat_id, text| *chat_id == CHAT_ID && text == "Linked ✓").await;

    let status_response = client
        .get(format!("{base}/telegram/link"))
        .header(reqwest::header::COOKIE, cookie)
        .send()
        .await
        .unwrap();
    let body: Value = status_response.json().await.unwrap();
    assert_eq!(body["linked"], true);

    let (chat_id,): (Option<i64>,) = sqlx::query_as("SELECT telegram_chat_id FROM users WHERE email = $1")
        .bind("telegram@test.com")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(chat_id, Some(CHAT_ID));
}

#[sqlx::test]
async fn unlinking_chat_clears_chat_id(pool: PgPool) {
    let (mock_addr, mock) = spawn_mock_telegram_api().await;
    let app_addr = start_app_with_mock_telegram_api(pool.clone(), &format!("http://{mock_addr}")).await;
    let base = format!("http://{app_addr}");
    let client = reqwest::Client::new();

    let cookie = link_chat_via_bot(&client, &base, &mock).await;

    let unlink_response = client
        .delete(format!("{base}/telegram/link"))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(unlink_response.status(), StatusCode::NO_CONTENT);

    let status_response = client
        .get(format!("{base}/telegram/link"))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await
        .unwrap();
    let body: Value = status_response.json().await.unwrap();
    assert_eq!(body["linked"], false);

    let (chat_id,): (Option<i64>,) = sqlx::query_as("SELECT telegram_chat_id FROM users WHERE email = $1")
        .bind("telegram@test.com")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(chat_id, None);
}

#[sqlx::test]
async fn cron_task_reminder_reaches_linked_chat(pool: PgPool) {
    let (mock_addr, mock) = spawn_mock_telegram_api().await;
    let app_addr = start_app_with_mock_telegram_api(pool.clone(), &format!("http://{mock_addr}")).await;
    let base = format!("http://{app_addr}");
    let client = reqwest::Client::new();

    let cookie = link_chat_via_bot(&client, &base, &mock).await;
    mock.sent_messages.lock().unwrap().clear();

    let create_task = client
        .post(format!("{base}/tasks"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&json!({"name": "Drink water", "cron": "* * * * * *", "active": true}))
        .send()
        .await
        .unwrap();
    assert_eq!(create_task.status(), StatusCode::CREATED);

    wait_for_bot_message(&mock, |chat_id, text| {
        *chat_id == CHAT_ID && text.contains("Drink water")
    })
    .await;
}

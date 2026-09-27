use reqwest::{Client, Proxy};
use serde_json::{Value, json};
use std::env;

const DEFAULT_API_BASE_URL: &str = "https://api.telegram.org";

#[derive(Clone, Debug)]
pub struct TelegramClient {
    pub client: Client,
    pub bot_token: String,
    pub bot_username: String,
    pub api_base_url: String,
}

impl TelegramClient {
    pub fn new(client: Client, bot_token: String, bot_username: String, api_base_url: String) -> Self {
        Self {
            client,
            bot_token,
            bot_username,
            api_base_url,
        }
    }

    pub fn from_env() -> Option<Self> {
        let bot_token = env::var("TELEGRAM_BOT_TOKEN")
            .ok()
            .filter(|token| !token.trim().is_empty())?;
        let client = match env::var("TELEGRAM_PROXY").ok() {
            Some(proxy_url) => Client::builder()
                .proxy(Proxy::all(&proxy_url).expect("invalid TELEGRAM_PROXY url"))
                .build()
                .expect("failed to build telegram http client"),
            None => Client::new(),
        };
        Some(Self {
            client,
            bot_token,
            bot_username: env::var("TELEGRAM_BOT_USERNAME").unwrap_or_default(),
            api_base_url: env::var("TELEGRAM_API_BASE_URL").unwrap_or_else(|_| DEFAULT_API_BASE_URL.to_string()),
        })
    }

    pub async fn request_updates(&self, offset: i64) -> Result<Vec<Message>, reqwest::Error> {
        let url = format!("{}/bot{}/getUpdates", self.api_base_url, self.bot_token);
        let body = self
            .client
            .get(&url)
            .query(&[("offset", offset), ("timeout", 50)])
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;
        let updates = body
            .get("result")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(updates.iter().filter_map(parse_message).collect())
    }

    pub async fn send_message(&self, chat_id: i64, text: &str) -> Result<(), reqwest::Error> {
        let url = format!("{}/bot{}/sendMessage", self.api_base_url, self.bot_token);
        self.client
            .post(url)
            .json(&json!({"chat_id": chat_id, "text": text}))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Message {
    pub id: i64,
    pub chat_id: i64,
    pub text: String,
}

fn parse_message(update: &Value) -> Option<Message> {
    let id = update.get("update_id").and_then(Value::as_i64)?;
    let message = update.get("message")?;
    let chat_id = message.get("chat")?.get("id")?.as_i64()?;
    let text = message.get("text")?.as_str()?.to_string();
    Some(Message { id, chat_id, text })
}

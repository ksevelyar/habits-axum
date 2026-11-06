# Notifications
## Requirements
* `GET /websocket/notifications` authenticates with a `jwt` cookie or `Authorization: Bearer` device token.
* A connected client first receives `UserAuthenticated`.
* Active tasks produce `TaskReminder` events at their cron times in the user's timezone.
* The server sends a WebSocket ping every 30 seconds and closes a connection that does not answer before the next ping.
* Telegram features require `TELEGRAM_BOT_TOKEN` and `TELEGRAM_BOT_USERNAME`. `TELEGRAM_PROXY` is optional.
* `GET`, `POST`, and `DELETE /telegram/link` require a session cookie.
* A user completes linking by sending `/link 6fdb21dd-7f0d-4651-86f8-0c30e10e6c19` to the bot.
* A reminder is delivered to linked Telegram chats before WebSocket broadcast.

## User scenarios
### 1. Connect to notifications
WebSocket request:
```http
GET /websocket/notifications
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjoiNmZkYjIxZGQtN2YwZC00NjUxLTg2ZjgtMGMzMGUxMGU2YzE5IiwiZGV2aWNlX25hbWUiOiJLaXRjaGVuIHRhYmxldCJ9.07trX4DfJROJfLdOwgvTOU2pUIXgdeET0hZKtyeb1c8
```

First event:
```json
{
  "event": "UserAuthenticated",
  "user": {
    "id": 1,
    "email": "person@example.com",
    "timezone": "Europe/London"
  }
}
```

### 2. Receive a task reminder
WebSocket request:
```http
GET /websocket/notifications
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjoiNmZkYjIxZGQtN2YwZC00NjUxLTg2ZjgtMGMzMGUxMGU2YzE5IiwiZGV2aWNlX25hbWUiOiJLaXRjaGVuIHRhYmxldCJ9.07trX4DfJROJfLdOwgvTOU2pUIXgdeET0hZKtyeb1c8
```

Event:
```json
{
  "event": "TaskReminder",
  "task_id": 7,
  "task_name": "Take a walk",
  "scheduled_time": "18:00"
}
```

### 3. Check Telegram link status
Request:
```http
GET /telegram/link
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

Response, `200 OK`:
```json
{
  "linked": false
}
```

### 4. Request a Telegram link
Request:
```http
POST /telegram/link
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

Response, `200 OK`:
```json
{
  "url": "https://t.me/habits_bot?start=6fdb21dd-7f0d-4651-86f8-0c30e10e6c19"
}
```

### 5. Report unavailable Telegram configuration
Request:
```http
GET /telegram/link
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

Response, `503 Service Unavailable`:
```json
{
  "error": "telegram is not configured"
}
```

# Notifications

## Websocket Notifications
### Flow
1. Client connects via WebSocket to `/websocket/notifications`
2. Auth via JWT (cookie `jwt` or `Authorization: Bearer`)
3. Server lazily creates a `tokio::sync::broadcast` channel plus a scheduler task per user
4. Scheduler loads active tasks from pg, finds the nearest cron fire time, sleeps until it
5. On fire: broadcasts `TaskReminder { task_id, task_name, scheduled_time }` to all connected clients
6. Scheduler exits when no active tasks remain, removing the user's channel from `state.channels`
7. Reconnecting client resumes — scheduler respawns on connect

### Heartbeat pings
* server sends ping every 30s and terminates connection if pong not received before next ping

### Connect
```
websocat -t - autoreconnect:ws://localhost:3003/websocket/notifications --header "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJleHAiOjE3ODA5MzY2NzcsImVtYWlsIjoia3NldmVseWFyQGdtYWlsLmNvbSIsImRldmljZV9pZCI6bnVsbCwiZGV2aWNlX25hbWUiOm51bGx9.ZDylN8H7MV0rV8Ya0ZYV-Iq0ny5NcI-MSkwaI3UER4A"

[INFO  websocat::net_peer] Connected to TCP 127.0.0.1:3003
[INFO  websocat::ws_client_peer] Connected to ws
{"event":"UserAuthenticated","user":{"email":"ksevelyar@gmail.com","id":1,"timezone":"Europe/Moscow"}}
```

## Telegram Notifications
### Config
* enabled when `TELEGRAM_BOT_TOKEN` env var is set
* `TELEGRAM_BOT_USERNAME` is required when the token is set, missing username panics at startup
* optional `TELEGRAM_PROXY`

### Linking
* client polls `/telegram/link` for status
* calls `/telegram/request-link` to get a bot deep link containing a uuid code
* user sends `/link <code>` to the bot, bot stores `telegram_chat_id` and calls `ensure_delivery`
* `/telegram/unlink` clears the chat id
* inbox worker long-polls `getUpdates` with 10s backoff on errors; the only command is `/link <code>`

### Delivery
* on each reminder fire, if the user has a linked chat, the reminder text (task name)
  is sent via `sendMessage` before the websocket broadcast

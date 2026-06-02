# Notifications

## Flow
1. Client connects via WebSocket to `/websocket/notifications`
2. Auth via JWT (cookie `jwt` or `Authorization: Bearer`)
3. Server lazily creates a `tokio::sync::broadcast` + a tokio scheduler task per user
4. Scheduler loads active tasks from pg, finds nearest cron fire time, sleeps until it
5. On fire: broadcasts `TaskReminder { task_id, task_name, scheduled_at }` to all connected clients
6. Scheduler dies after 3 consecutive ticks with zero receivers, or when no active tasks remain
7. Reconnecting client resumes — scheduler restarts on connect if dead

## Connect
```
websocat -t - autoreconnect:ws://localhost:3003/websocket/notifications --header "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJleHAiOjE3ODA5MzY2NzcsImVtYWlsIjoia3NldmVseWFyQGdtYWlsLmNvbSIsImRldmljZV9pZCI6bnVsbCwiZGV2aWNlX25hbWUiOm51bGx9.ZDylN8H7MV0rV8Ya0ZYV-Iq0ny5NcI-MSkwaI3UER4A"

[INFO  websocat::net_peer] Connected to TCP 127.0.0.1:3003
[INFO  websocat::ws_client_peer] Connected to ws
{"event":"UserAuthenticated","user":{"email":"ksevelyar@gmail.com","id":1,"timezone":"Europe/Moscow"}}
```

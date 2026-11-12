# WebSocket notifications
## Requirements
* Users receive reminders for active tasks at their scheduled local times.
* Users receive notifications in connected clients.
* A user only receives their own reminders.

## User scenarios
### 1. A user connects to notifications
WebSocket request:
```http
GET /websocket/notifications
Authorization: Bearer <jwt>
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

### 2. A user receives a task reminder
WebSocket request:
```http
GET /websocket/notifications
Authorization: Bearer <jwt>
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

### 3. A user connects to notifications with a cookie
WebSocket request:
```http
GET /websocket/notifications
Cookie: jwt=<jwt>
```

First event: same `UserAuthenticated` event as scenario 1.

### 4. A user receives the last due task reminder when connecting
On connect, after `UserAuthenticated`, the server replays the most recent due `TaskReminder` for the user's local time.

### 5. A user cannot connect to notifications with an invalid token
WebSocket request with an invalid jwt is rejected and the connection fails.

### 6. A user receives reminders for all due active tasks
Each active task with a due time sends its own `TaskReminder` event.

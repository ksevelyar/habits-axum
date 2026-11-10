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

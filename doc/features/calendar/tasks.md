# Tasks
## Requirements
* Users create and manage reminders with a name and schedule.
* New reminders are inactive until enabled.
* Active reminders notify the user at their scheduled times.
* A user only sees and changes their own reminders.

## User scenarios
### 1. A user creates an active reminder
Request:
```http
POST /tasks
Cookie: jwt=<jwt>
```

```json
{
  "name": "Take a walk",
  "cron": "0 18 * * *",
  "active": true
}
```

Response, `201 Created`:
```json
{
  "id": 7,
  "name": "Take a walk",
  "active": true,
  "cron": "0 18 * * *",
  "user_id": 1,
  "inserted_at": "2026-03-09T12:00:00Z",
  "updated_at": "2026-03-09T12:00:00Z"
}
```

### 2. A user updates a reminder
Request:
```http
PATCH /tasks/7
Cookie: jwt=<jwt>
```

```json
{
  "active": false
}
```

Response, `200 OK`:
```json
{
  "id": 7,
  "name": "Take a walk",
  "active": false,
  "cron": "0 18 * * *",
  "user_id": 1,
  "inserted_at": "2026-03-09T12:00:00Z",
  "updated_at": "2026-03-09T13:00:00Z"
}
```

### 3. A user cannot create a task without a cron expression
Request:
```http
POST /tasks
Cookie: jwt=<jwt>
```

```json
{
  "name": "Take a walk"
}
```

Response, `422 Unprocessable Entity`:
```json
{
  "error": "validation failed",
  "fields": [
    {
      "field": "cron",
      "message": "cron is required"
    }
  ]
}
```

### 4. A user cannot access another user's task
Request:
```http
GET /tasks/7
Cookie: jwt=<jwt>
```

Response, `404 Not Found`:
```json
{
  "error": "task not found"
}
```

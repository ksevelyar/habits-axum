# Telegram notifications
## Requirements
* Users can link and unlink their Telegram account to receive reminders there.
* A user only receives their own reminders.

## User scenarios
### 1. A user checks Telegram link status
Request:
```http
GET /telegram/link
Cookie: jwt=<jwt>
```

Response, `200 OK`:
```json
{
  "linked": false
}
```

### 2. A user requests a Telegram link
Request:
```http
POST /telegram/link
Cookie: jwt=<jwt>
```

Response, `200 OK`:
```json
{
  "url": "https://t.me/habits_bot?start=6fdb21dd-7f0d-4651-86f8-0c30e10e6c19"
}
```

### 3. A user receives an error when Telegram configuration is unavailable
Request:
```http
GET /telegram/link
Cookie: jwt=<jwt>
```

Response, `503 Service Unavailable`:
```json
{
  "error": "telegram is not configured"
}
```

### 4. A user links a Telegram chat with a bot code
The user submits `/link <code>` to the bot in their chat, the app stores the chat id and replies "Linked ✓".

### 5. A user unlinks a Telegram chat
Request:
```http
DELETE /telegram/link
Cookie: jwt=<jwt>
```

Response, `204 No Content`: the stored chat id is cleared.

### 6. A user receives an active reminder in a linked Telegram chat
When a cron task fires for a linked user, the bot sends the task name to the linked chat.

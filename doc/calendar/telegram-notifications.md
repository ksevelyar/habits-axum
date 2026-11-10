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

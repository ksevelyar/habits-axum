# Tasks
## Requirements
* All task endpoints require a session cookie.
* A task belongs to the authenticated user.
* Creating a task requires `name` and `cron`. `active` defaults to `false`.
* Updates accept any subset of `name`, `cron`, and `active`.
* Activating a task ensures reminder delivery is scheduled.
* Task indexes are ordered by descending id.

## User scenarios
### 1. Create an active reminder
Request:
```http
POST /tasks
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
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

### 2. Update a reminder
Request:
```http
PATCH /tasks/7
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
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

### 3. Reject a task without a cron expression
Request:
```http
POST /tasks
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
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

### 4. Hide another user's task
Request:
```http
GET /tasks/7
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoiYW5vdGhlckBleGFtcGxlLmNvbSIsImRldmljZV9pZCI6bnVsbCwiZGV2aWNlX25hbWUiOm51bGx9.D3r477qnfxD90REBgpQ1s6AMDa-JQ0H_z7TyYJ67fAY
```

Response, `404 Not Found`:
```json
{
  "error": "task not found"
}
```

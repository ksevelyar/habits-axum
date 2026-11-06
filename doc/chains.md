# Chains
## Requirements
* All chain endpoints require a session cookie.
* A chain belongs to the authenticated user.
* `type` is `integer`, `float`, `boolean`, or `time`.
* `aggregate` is `sum` or `avg`.
* New chains require `active`, `aggregate`, `name`, and `type`. `show_in_charts` defaults to `false`.
* Updates accept any subset of chain fields.
* Chain indexes are ordered by descending id.

## User scenarios
### 1. Create a chain
Request:
```http
POST /chains
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

```json
{
  "active": true,
  "aggregate": "sum",
  "show_in_charts": false,
  "name": "Push-ups",
  "type": "integer",
  "description": "Daily total",
  "order": 1
}
```

Response, `201 Created`:
```json
{
  "id": 42,
  "user_id": 1,
  "active": true,
  "show_in_charts": false,
  "name": "Push-ups",
  "type": "integer",
  "aggregate": "sum",
  "description": "Daily total",
  "order": 1,
  "inserted_at": "2026-03-09T12:00:00Z",
  "updated_at": "2026-03-09T12:00:00Z"
}
```

### 2. Enable a chain in charts
Request:
```http
PATCH /chains/42
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

```json
{
  "show_in_charts": true
}
```

Response, `200 OK`:
```json
{
  "id": 42,
  "user_id": 1,
  "active": true,
  "show_in_charts": true,
  "name": "Push-ups",
  "type": "integer",
  "aggregate": "sum",
  "description": "Daily total",
  "order": 1,
  "inserted_at": "2026-03-09T12:00:00Z",
  "updated_at": "2026-03-09T12:10:00Z"
}
```

### 3. Read a chain
Request:
```http
GET /chains/42
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

Response, `200 OK`:
```json
{
  "id": 42,
  "user_id": 1,
  "active": true,
  "show_in_charts": true,
  "name": "Push-ups",
  "type": "integer",
  "aggregate": "sum",
  "description": "Daily total",
  "order": 1,
  "inserted_at": "2026-03-09T12:00:00Z",
  "updated_at": "2026-03-09T12:10:00Z"
}
```

### 4. Hide another user's chain
Request:
```http
GET /chains/42
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoiYW5vdGhlckBleGFtcGxlLmNvbSIsImRldmljZV9pZCI6bnVsbCwiZGV2aWNlX25hbWUiOm51bGx9.D3r477qnfxD90REBgpQ1s6AMDa-JQ0H_z7TyYJ67fAY
```

Response, `404 Not Found`:
```json
{
  "error": "chain not found"
}
```

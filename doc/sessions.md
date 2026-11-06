# Sessions
## Requirements
* `POST /sessions` authenticates an email and password.
* A successful session sets an HTTP-only `jwt` cookie scoped to `/` for one year.
* The cookie is secure outside debug builds.
* `GET /sessions/current` returns the authenticated user.
* Missing, invalid, expired, and deleted-user sessions are unauthorized.
* `JWT_SECRET` signs and verifies session tokens.

## User scenarios
### 1. Sign in
Request:
```http
POST /sessions
```

```json
{
  "email": "person@example.com",
  "password": "correct horse battery staple"
}
```

Response, `201 Created`:
```json
{
  "set_cookie": {
    "name": "jwt",
    "http_only": true,
    "path": "/",
    "max_age_seconds": 31536000
  }
}
```

### 2. Read the signed-in user
Request:
```http
GET /sessions/current
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

Response, `200 OK`:
```json
{
  "id": 1,
  "email": "person@example.com",
  "timezone": "Europe/London"
}
```

### 3. Reject invalid credentials
Request:
```http
POST /sessions
```

```json
{
  "email": "person@example.com",
  "password": "incorrect password"
}
```

Response, `401 Unauthorized`:
```json
{
  "error": "unauthorized"
}
```

### 4. Report missing session configuration
Request:
```http
POST /sessions
```

```json
{
  "email": "person@example.com",
  "password": "correct horse battery staple"
}
```

Response, `503 Service Unavailable`:
```json
{
  "error": "JWT_SECRET is not set"
}
```

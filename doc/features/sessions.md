# Sessions
## Requirements
* Users sign in with their email address and password.
* A sign-in remains active for one year.

## User scenarios
### 1. A user signs in
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

### 2. A signed-in user reads their account
Request:
```http
GET /sessions/current
Cookie: jwt=<jwt>
```

Response, `200 OK`:
```json
{
  "id": 1,
  "email": "person@example.com",
  "timezone": "Europe/London"
}
```

### 3. A user cannot sign in with invalid credentials
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

### 4. A user receives an error when session configuration is missing
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

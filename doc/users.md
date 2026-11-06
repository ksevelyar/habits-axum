# Users
## Requirements
* `POST /users` creates an account without authentication.
* Email, handle, password, and IANA timezone are required.
* Email and handle are unique. Email uniqueness is case-insensitive.
* Public user responses exclude passwords and password hashes.

## User scenarios
### 1. Register an account
Request:
```http
POST /users
```

```json
{
  "email": "person@example.com",
  "handle": "person",
  "password": "correct horse battery staple",
  "timezone": "Europe/London"
}
```

Response, `201 Created`:
```json
{
  "id": 1,
  "email": "person@example.com",
  "timezone": "Europe/London"
}
```

### 2. Reject an email already in use
Request:
```http
POST /users
```

```json
{
  "email": "PERSON@example.com",
  "handle": "another-person",
  "password": "correct horse battery staple",
  "timezone": "Europe/London"
}
```

Response, `422 Unprocessable Entity`:
```json
{
  "error": "validation failed",
  "fields": [
    {
      "field": "email",
      "message": "email is already taken"
    }
  ]
}
```

### 3. Reject an unknown timezone
Request:
```http
POST /users
```

```json
{
  "email": "person@example.com",
  "handle": "person",
  "password": "correct horse battery staple",
  "timezone": "Mars/Olympus_Mons"
}
```

Response, `422 Unprocessable Entity`:
```json
{
  "error": "validation failed",
  "fields": [
    {
      "field": "timezone",
      "message": "unknown timezone"
    }
  ]
}
```

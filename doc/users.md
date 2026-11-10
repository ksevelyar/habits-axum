# Users
## Requirements
* People create an account with an email address, handle, password, and timezone.
* Account information never exposes passwords.

## User scenarios
### 1. A user registers an account
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

### 2. A user cannot register with an email already in use
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

### 3. A user cannot register with an unknown timezone
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

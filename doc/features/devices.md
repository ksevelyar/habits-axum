# Devices
## Requirements
* Users connect named devices to their account.
* Device tokens do not expire.
* Connected devices can record metrics and receive notifications for their owner.

## User scenarios
### 1. A user creates a device token
Request:
```http
POST /devices
Cookie: jwt=<jwt>
```

```json
{
  "device_name": "Kitchen tablet"
}
```

Response, `201 Created`:
```json
{
  "device_id": "6fdb21dd-7f0d-4651-86f8-0c30e10e6c19",
  "device_name": "Kitchen tablet",
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjoiNmZkYjIxZGQtN2YwZC00NjUxLTg2ZjgtMGMzMGUxMGU2YzE5IiwiZGV2aWNlX25hbWUiOiJLaXRjaGVuIHRhYmxldCJ9.07trX4DfJROJfLdOwgvTOU2pUIXgdeET0hZKtyeb1c8"
}
```

### 2. A user cannot create a device token without a device name
Request:
```http
POST /devices
Cookie: jwt=<jwt>
```

```json
{}
```

Response, `422 Unprocessable Entity`.

### 3. A user without a session cannot create a device token
Request:
```http
POST /devices
```

```json
{
  "device_name": "Kitchen tablet"
}
```

Response, `401 Unauthorized`:
```json
{
  "error": "unauthorized"
}
```

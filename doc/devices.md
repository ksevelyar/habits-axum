# Devices
## Requirements
* `POST /devices` requires a cookie session.
* A device token identifies the authenticated user, a generated device id, and the supplied device name.
* Device tokens do not expire.
* Metric writes and notification WebSocket connections accept a device token through `Authorization: Bearer`.

## User scenarios
### 1. Create a device token
Request:
```http
POST /devices
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
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

### 2. Reject device creation without a session
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

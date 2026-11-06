# Metrics
## Requirements
* Metric writes accept a session cookie or a device bearer token. Reads use a session cookie.
* A metric belongs to a chain owned by the authenticated user.
* A chain has at most one metric per date. `POST /metrics` creates or updates it.
* Metric values are strings. Integer and time chains require a nonzero integer, float chains a nonzero float, and boolean chains require `true`.
* Date lookups return every active chain, including chains without a metric on that date.
* Metric history returns active chains and the current and previous Monday-to-Sunday sprints.

## User scenarios
### 1. Record an integer metric
Request:
```http
POST /metrics
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjoiNmZkYjIxZGQtN2YwZC00NjUxLTg2ZjgtMGMzMGUxMGU2YzE5IiwiZGV2aWNlX25hbWUiOiJLaXRjaGVuIHRhYmxldCJ9.07trX4DfJROJfLdOwgvTOU2pUIXgdeET0hZKtyeb1c8
```

```json
{
  "chain_id": 42,
  "date": "2026-03-09",
  "value": "25"
}
```

Response, `200 OK`:
```json
{
  "id": 9,
  "chain_id": 42,
  "date": "2026-03-09",
  "value_integer": 25,
  "value_float": null,
  "value_bool": null
}
```

### 2. Read metrics for one date
Request:
```http
GET /metrics?date=2026-03-09
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

Response, `200 OK`:
```json
[
  {
    "id": 9,
    "value": 25.0,
    "updated_at": "2026-03-09T12:00:00Z",
    "chain": "Push-ups",
    "chain_id": 42
  },
  {
    "id": null,
    "value": null,
    "updated_at": null,
    "chain": "Meditate",
    "chain_id": 43
  }
]
```

### 3. Reject a zero metric
Request:
```http
POST /metrics
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

```json
{
  "chain_id": 42,
  "date": "2026-03-09",
  "value": "0"
}
```

Response, `400 Bad Request`:
```json
{
  "error": "metric value cannot be zero"
}
```

### 4. Reject a metric for another user's chain
Request:
```http
POST /metrics
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

```json
{
  "chain_id": 42,
  "date": "2026-03-09",
  "value": "25"
}
```

Response, `404 Not Found`:
```json
{
  "error": "chain not found"
}
```

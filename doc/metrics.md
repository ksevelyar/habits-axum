# Metrics
## Requirements
* Users record one value per chain per day and can correct it later.
* Users can review their active chains and recorded values for the current and previous week.
* A user only sees and records values for their own chains.

## User scenarios
### 1. A user records an integer metric
Request:
```http
POST /metrics
Cookie: jwt=<jwt>
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

### 2. A user reads metrics for one date
Request:
```http
GET /metrics?date=2026-03-09
Cookie: jwt=<jwt>
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

### 3. A user cannot record a zero metric
Request:
```http
POST /metrics
Cookie: jwt=<jwt>
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

### 4. A user cannot record a metric for another user's chain
Request:
```http
POST /metrics
Cookie: jwt=<jwt>
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

# Charts
## Requirements
* `GET /charts` requires a session cookie.
* Chart history includes only the authenticated user's active chains enabled for charts.
* `start_date` and `end_date` are required inclusive ISO dates.
* Chains are ordered by explicit order, then id. Points are ordered by date.
* Each chain includes its type, aggregate, and every metric within the requested range.

## User scenarios
### 1. Enable a chain in charts
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
  "name": "Pull-ups",
  "type": "integer",
  "aggregate": "sum",
  "description": null,
  "order": 1,
  "inserted_at": "2026-02-09T00:00:00Z",
  "updated_at": "2026-03-09T00:00:00Z"
}
```

### 2. Load one month of chart history
Request:
```http
GET /charts?start_date=2026-02-09&end_date=2026-03-09
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

Response, `200 OK`:
```json
{
  "chains": [
    {
      "id": 42,
      "name": "Pull-ups",
      "type": "integer",
      "aggregate": "sum",
      "points": [
        {
          "date": "2026-02-09",
          "value": 20.0
        },
        {
          "date": "2026-02-11",
          "value": 25.0
        }
      ]
    }
  ]
}
```

### 3. Return no chains when none are enabled
Request:
```http
GET /charts?start_date=2026-02-09&end_date=2026-03-09
Cookie: jwt=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjQxMDI0NDQ4MDAsImVtYWlsIjoicGVyc29uQGV4YW1wbGUuY29tIiwiZGV2aWNlX2lkIjpudWxsLCJkZXZpY2VfbmFtZSI6bnVsbH0.K_wZSRP7f6f-fJayUpWmFDbwLbt6e-Af_538M5Ht_U4
```

Response, `200 OK`:
```json
{
  "chains": []
}
```

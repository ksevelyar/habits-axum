# Charts
## Requirements
* Users view charts for their active chains that they chose to show in charts.
* Users select the date range shown in a chart.
* A chart shows each chain's values and aggregation for the selected period.
* A user only sees their own charts.

## User scenarios
### 1. A user loads one month of chart history
Request:
```http
GET /charts?start_date=2026-03-01&end_date=2026-04-01
Cookie: jwt=<jwt>
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
          "date": "2026-03-05",
          "value": 20.0
        },
        {
          "date": "2026-03-07",
          "value": 25.0
        }
      ]
    }
  ]
}
```

### 2. A user sees no chains when none are enabled
Request:
```http
GET /charts?start_date=2026-03-01&end_date=2026-04-01
Cookie: jwt=<jwt>
```

Response, `200 OK`:
```json
{
  "chains": []
}
```

### 3. An unauthenticated user cannot load chart history
Request:
```http
GET /charts?start_date=2026-03-01&end_date=2026-04-01
```

Response, `401 Unauthorized`.

### 4. A user only sees their active enabled chains
Request:
```http
GET /charts?start_date=2026-03-01&end_date=2026-04-01
Cookie: jwt=<jwt>
```

Response, `200 OK`: chains owned by the user that are active and shown in charts, other users' chains are absent.

### 5. A user does not see metrics at the end of a chart range
Request:
```http
GET /charts?start_date=2026-02-09&end_date=2026-02-13
Cookie: jwt=<jwt>
```

Response, `200 OK`: points include the start date but exclude the end date.

### 6. A user cannot load an empty or reversed chart range
Request:
```http
GET /charts?start_date=2026-03-01&end_date=2026-03-01
Cookie: jwt=<jwt>
```

Response, `400 Bad Request`:
```json
{
  "error": "start_date must be before end_date"
}
```

# Charts
## Requirements
* Users view charts for their active chains that they chose to show in charts.
* Users select the date range shown in a chart.
* A chart shows each chain's values and aggregation for the selected period.
* A user only sees their own charts.

## User scenarios
### 1. A user enables a chain in charts
Request:
```http
PATCH /chains/42
Cookie: jwt=<jwt>
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

### 2. A user loads one month of chart history
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

### 3. A user sees no chains when none are enabled
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

# Charts

## Requirements
* Chart history contains active chains enabled for charts.
* Chart history contains daily metric values from start_date through end_date, inclusive.
* Chart history belongs to the authenticated user.

## User scenarios

### Enable Pull-ups
```http
PATCH /chains/42
Content-Type: application/json

{
  "show_in_charts": true
}
```

```json
{
  "id": 42,
  "name": "Pull-ups",
  "show_in_charts": true
}
```

### Load one month of Pull-ups
```http
GET /charts?start_date=2026-02-09&end_date=2026-03-09
```

```json
{
  "chains": [
    {
      "id": 42,
      "name": "Pull-ups",
      "type": "integer",
      "points": [
        { "date": "2026-02-09", "value": 20 },
        { "date": "2026-02-11", "value": 25 }
      ]
    }
  ]
}
```

# Chains
## Requirements
* Users create and manage their own tracking chains.
* A chain tracks integer, decimal, yes-or-no, or duration values.
* A chain totals or averages its recorded values.
* Users choose whether a chain is active and appears in charts.
* A user only sees and changes their own chains.

## User scenarios
### 1. A user creates a chain
Request:
```http
POST /chains
Cookie: jwt=<jwt>
```

```json
{
  "active": true,
  "aggregate": "sum",
  "show_in_charts": false,
  "name": "Push-ups",
  "type": "integer",
  "description": "Daily total",
  "order": 1
}
```

Response, `201 Created`:
```json
{
  "id": 42,
  "user_id": 1,
  "active": true,
  "show_in_charts": false,
  "name": "Push-ups",
  "type": "integer",
  "aggregate": "sum",
  "description": "Daily total",
  "order": 1,
  "inserted_at": "2026-03-09T12:00:00Z",
  "updated_at": "2026-03-09T12:00:00Z"
}
```

### 2. A user enables a chain in charts
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
  "name": "Push-ups",
  "type": "integer",
  "aggregate": "sum",
  "description": "Daily total",
  "order": 1,
  "inserted_at": "2026-03-09T12:00:00Z",
  "updated_at": "2026-03-09T12:10:00Z"
}
```

### 3. A user reads a chain
Request:
```http
GET /chains/42
Cookie: jwt=<jwt>
```

Response, `200 OK`:
```json
{
  "id": 42,
  "user_id": 1,
  "active": true,
  "show_in_charts": true,
  "name": "Push-ups",
  "type": "integer",
  "aggregate": "sum",
  "description": "Daily total",
  "order": 1,
  "inserted_at": "2026-03-09T12:00:00Z",
  "updated_at": "2026-03-09T12:10:00Z"
}
```

### 4. A user cannot read another user's chain
Request:
```http
GET /chains/42
Cookie: jwt=<jwt>
```

Response, `404 Not Found`:
```json
{
  "error": "chain not found"
}
```

### 5. A user cannot create a chain with invalid input
Request:
```http
POST /chains
Cookie: jwt=<jwt>
```

```json
{"active": true}
```

Response, `422 Unprocessable Entity`.

### 6. A user cannot update a chain with invalid input
Request:
```http
PATCH /chains/42
Cookie: jwt=<jwt>
```

```json
{"type": "bad"}
```

Response, `422 Unprocessable Entity`.

### 7. A user deletes a chain
Request:
```http
DELETE /chains/42
Cookie: jwt=<jwt>
```

Response, `204 No Content`.

### 8. A user lists their chains
Request:
```http
GET /chains
Cookie: jwt=<jwt>
```

Response, `200 OK`: a JSON array of the user's chains, newest first.

# Ledger transactions
## Requirements
* Users record income and expenses in their categories.
* An amount cannot be zero. Positive amounts are income and negative amounts are expenses.
* Users can correct or remove recorded transactions.
* Users can review transactions for a selected period and see their balance at the end of that period.
* A user only sees and changes their own transactions and categories.

## User scenarios
### 1. A user records work income
The client converts `+1500` rubles to `150000` kopecks before sending the request.

Request:
```http
POST /ledger/transactions
Cookie: jwt=<jwt>
Content-Type: application/json
```

```json
{
  "category_id": 1,
  "amount_kopecks": 150000,
  "occurred_at": "2026-03-09T12:00:00Z"
}
```

Response, `201 Created`:
```json
{
  "id": 1,
  "category_id": 1,
  "amount_kopecks": 150000,
  "occurred_at": "2026-03-09T12:00:00.000000Z",
  "inserted_at": "2026-03-09T12:00:00.000000Z",
  "updated_at": "2026-03-09T12:00:00.000000Z"
}
```

### 2. A user records home spending
The client converts `-1500` rubles to `-150000` kopecks before sending the request.

Request:
```http
POST /ledger/transactions
Cookie: jwt=<jwt>
Content-Type: application/json
```

```json
{
  "category_id": 2,
  "amount_kopecks": -150000,
  "occurred_at": "2026-03-09T18:30:00Z"
}
```

Response, `201 Created`:
```json
{
  "id": 2,
  "category_id": 2,
  "amount_kopecks": -150000,
  "occurred_at": "2026-03-09T18:30:00.000000Z",
  "inserted_at": "2026-03-09T18:31:00.000000Z",
  "updated_at": "2026-03-09T18:31:00.000000Z"
}
```

### 3. A user must provide a category
Request:
```http
POST /ledger/transactions
Cookie: jwt=<jwt>
Content-Type: application/json
```

```json
{
  "amount_kopecks": -100,
  "occurred_at": "2026-03-09T12:00:00Z"
}
```

Response, `422 Unprocessable Entity`:
```json
{
  "error": "validation failed",
  "fields": [
    {
      "field": "category_id",
      "message": "missing field `category_id`"
    }
  ]
}
```

### 4. A user must provide an occurrence timestamp
Request:
```http
POST /ledger/transactions
Cookie: jwt=<jwt>
Content-Type: application/json
```

```json
{
  "category_id": 2,
  "amount_kopecks": -100
}
```

Response, `422 Unprocessable Entity`:
```json
{
  "error": "validation failed",
  "fields": [
    {
      "field": "occurred_at",
      "message": "missing field `occurred_at`"
    }
  ]
}
```

### 5. A user cannot submit a zero amount
Request:
```http
POST /ledger/transactions
Cookie: jwt=<jwt>
Content-Type: application/json
```

```json
{
  "category_id": 2,
  "amount_kopecks": 0,
  "occurred_at": "2026-03-09T12:00:00Z"
}
```

Response, `400 Bad Request`:
```json
{
  "error": "amount_kopecks cannot be zero"
}
```

### 6. A user cannot submit a timestamp that is not UTC
Request:
```http
POST /ledger/transactions
Cookie: jwt=<jwt>
Content-Type: application/json
```

```json
{
  "category_id": 2,
  "amount_kopecks": -100,
  "occurred_at": "2026-03-09T15:00:00+03:00"
}
```

Response, `400 Bad Request`:
```json
{
  "error": "occurred_at must be a UTC RFC 3339 timestamp ending in Z"
}
```

### 7. A user cannot create a transaction in another user's category
Request:
```http
POST /ledger/transactions
Cookie: jwt=<jwt>
Content-Type: application/json
```

```json
{
  "category_id": 42,
  "amount_kopecks": 150000,
  "occurred_at": "2026-03-09T12:00:00Z"
}
```

Response, `404 Not Found`:
```json
{
  "error": "category not found"
}
```

### 8. A user updates a transaction
Request:
```http
PATCH /ledger/transactions/9
Cookie: jwt=<jwt>
Content-Type: application/json
```

```json
{
  "category_id": 3,
  "amount_kopecks": -1200,
  "occurred_at": "2026-03-10T14:30:00Z"
}
```

Response, `200 OK`:
```json
{
  "id": 9,
  "category_id": 3,
  "amount_kopecks": -1200,
  "occurred_at": "2026-03-10T14:30:00.000000Z",
  "inserted_at": "2026-03-09T12:00:00.000000Z",
  "updated_at": "2026-03-10T14:30:00.000000Z"
}
```

### 9. A user cannot update a transaction with a zero amount
Request:
```http
PATCH /ledger/transactions/9
Cookie: jwt=<jwt>
Content-Type: application/json
```

```json
{
  "category_id": 3,
  "amount_kopecks": 0,
  "occurred_at": "2026-03-10T14:30:00Z"
}
```

Response, `400 Bad Request`:
```json
{
  "error": "amount_kopecks cannot be zero"
}
```

### 10. A user cannot update a transaction with a non-UTC timestamp
Request:
```http
PATCH /ledger/transactions/9
Cookie: jwt=<jwt>
Content-Type: application/json
```

```json
{
  "category_id": 3,
  "amount_kopecks": -1200,
  "occurred_at": "2026-03-10T14:30:00+03:00"
}
```

Response, `400 Bad Request`:
```json
{
  "error": "occurred_at must be a UTC RFC 3339 timestamp ending in Z"
}
```

### 11. A user deletes a transaction
Request:
```http
DELETE /ledger/transactions/9
Cookie: jwt=<jwt>
```

Response, `204 No Content`.

### 12. A user lists an explicit UTC interval
For March 2026 in `Europe/Moscow`, the client resolves local midnight boundaries and sends them as UTC timestamps.

Request:
```http
GET /ledger/transactions?start_at=2026-02-28T21%3A00%3A00.000Z&end_at=2026-03-31T21%3A00%3A00.000Z
Cookie: jwt=<jwt>
```

Response, `200 OK`:
```json
{
  "start_at": "2026-02-28T21:00:00Z",
  "end_at": "2026-03-31T21:00:00Z",
  "balance_kopecks": 150000,
  "transactions": [
    {
      "id": 9,
      "category_id": 1,
      "category_name": "Work",
      "amount_kopecks": 150000,
      "occurred_at": "2026-03-09T12:00:00.000000Z",
      "inserted_at": "2026-03-09T12:00:00.000000Z",
      "updated_at": "2026-03-09T12:00:00.000000Z"
    }
  ]
}
```

### 13. An unauthenticated user cannot list transactions
Request:
```http
GET /ledger/transactions?start_at=2026-02-28T21%3A00%3A00.000Z&end_at=2026-03-31T21%3A00%3A00.000Z
```

Response, `401 Unauthorized`:
```json
{
  "error": "unauthorized"
}
```

### 14. A user must provide every field when updating a transaction
Request:
```http
PATCH /ledger/transactions/1
Cookie: jwt=<jwt>
```

```json
{"category_id": 1, "amount_kopecks": -1200}
```

Response, `422 Unprocessable Entity`: a field error for the missing `occurred_at`.

### 15. A user cannot update a transaction to another user's category
Request:
```http
PATCH /ledger/transactions/1
Cookie: jwt=<jwt>
```

Response, `404 Not Found`:
```json
{
  "error": "transaction not found"
}
```

### 16. A user lists only their transactions
Request:
```http
GET /ledger/transactions?start_at=...&end_at=...
Cookie: jwt=<jwt>
```

Response, `200 OK`: only transactions whose category belongs to the signed-in user.

### 17. A user sees transactions at the start of an interval but not the end
Transactions at `start_at` are included, transactions at `end_at` are excluded.

### 18. A user cannot list transactions with invalid interval bounds
Invalid, non-UTC, or reversed `start_at` and `end_at` respond `400 Bad Request` with an error message for the offending bound.

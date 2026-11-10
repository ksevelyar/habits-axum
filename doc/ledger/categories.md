# Ledger categories
## Requirements
* Users create and rename categories for their ledger.
* A category name is required and unique within a user's ledger.
* A category may contain both income and expense transactions.
* Users cannot remove a category that contains transactions.
* A user only sees and changes their own categories.

## User scenarios
### 1. A user creates a category
Request:
```http
POST /ledger/categories
Cookie: jwt=<jwt>
Content-Type: application/json
```

```json
{
  "name": "Work"
}
```

Response, `201 Created`:
```json
{
  "id": 1,
  "user_id": 1,
  "name": "Work",
  "inserted_at": "2026-03-09T12:00:00.000000Z",
  "updated_at": "2026-03-09T12:00:00.000000Z"
}
```

### 2. An authenticated user lists their categories
Request:
```http
GET /ledger/categories
Cookie: jwt=<jwt>
```

Response, `200 OK`:
```json
[
  {
    "id": 1,
    "user_id": 1,
    "name": "Work",
    "inserted_at": "2026-03-09T12:00:00.000000Z",
    "updated_at": "2026-03-09T12:00:00.000000Z"
  },
  {
    "id": 2,
    "user_id": 1,
    "name": "Home",
    "inserted_at": "2026-03-09T12:01:00.000000Z",
    "updated_at": "2026-03-09T12:01:00.000000Z"
  }
]
```

### 3. A user renames a category
Request:
```http
PATCH /ledger/categories/2
Cookie: jwt=<jwt>
Content-Type: application/json
```

```json
{
  "name": "Home"
}
```

Response, `200 OK`:
```json
{
  "id": 2,
  "user_id": 1,
  "name": "Home",
  "inserted_at": "2026-03-09T12:01:00.000000Z",
  "updated_at": "2026-03-09T12:10:00.000000Z"
}
```

### 4. A user deletes an unused category
Request:
```http
DELETE /ledger/categories/2
Cookie: jwt=<jwt>
```

Response, `204 No Content`.

### 5. A user cannot delete a category with transactions
Request:
```http
DELETE /ledger/categories/2
Cookie: jwt=<jwt>
```

Response, `400 Bad Request`:
```json
{
  "error": "category has transactions"
}
```

### 6. A user cannot access another user's category
Request:
```http
GET /ledger/categories
Cookie: jwt=<jwt>
```

Response, `200 OK`:
```json
[
  {
    "id": 1,
    "user_id": 1,
    "name": "Work",
    "inserted_at": "2026-03-09T12:00:00.000000Z",
    "updated_at": "2026-03-09T12:00:00.000000Z"
  }
]
```

A request to rename another user's category returns `404 Not Found`:
```http
PATCH /ledger/categories/7
Cookie: jwt=<jwt>
Content-Type: application/json
```

```json
{
  "name": "Home"
}
```

```json
{
  "error": "category not found"
}
```

### 7. An unauthenticated user cannot access categories
Request:
```http
GET /ledger/categories
```

Response, `401 Unauthorized`:
```json
{
  "error": "unauthorized"
}
```

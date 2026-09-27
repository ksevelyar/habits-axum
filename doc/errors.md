# Errors

## Contract
Contexts are the app's public API. Their fallible functions return `Result<T, AppError>`. Raw errors (`sqlx::Error`, `reqwest::Error`) stay inside contexts as implementation details.

## Types
`AppError` outcomes:

* `Unauthorized`: missing, expired, or invalid credentials
* `NotFound(String)`: requested entity absent or owned by another user
* `Validation(Vec<FieldError>)`: malformed payload, per-field messages
* `BadRequest(String)`: unparseable client input
* `ApplicationError(String)`: application or infrastructure fault
* `ConfigError(String)`: optional feature not configured

## Rules
* Repos log failures via `#[tracing::instrument(err)]` and propagate raw errors untouched.
* `IntoResponse` renders status and body only
* `ApplicationError` renders a generic body regardless of its message.

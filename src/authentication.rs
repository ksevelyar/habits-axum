use axum::http::HeaderMap;
use axum_extra::extract::cookie::Cookie;
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub use bcrypt::verify;

use crate::error::AppError;
use crate::users::User;

#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    pub exp: u64,
    pub email: String,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
}

const SESSION_DURATION_SECONDS: u64 = 7 * 24 * 3600;

pub async fn authenticate_cookie(
    pool: &PgPool,
    cookie_jar: &axum_extra::extract::cookie::CookieJar,
) -> Result<User, AppError> {
    let jwt = cookie_jar.get("jwt").ok_or(AppError::Unauthorized)?.value();
    authenticate_token(pool, jwt).await
}

pub async fn authenticate_token(pool: &PgPool, token: &str) -> Result<User, AppError> {
    let token_data = decode_jwt(token).map_err(|_| AppError::Unauthorized)?;
    sqlx::query_as::<_, User>("SELECT id, email, timezone FROM users WHERE email = $1")
        .bind(token_data.claims.email)
        .fetch_one(pool)
        .await
        .map_err(|err| {
            tracing::error!("{err}");
            AppError::Unauthorized
        })
}

pub fn encode_jwt(email: String) -> Result<String, axum::http::StatusCode> {
    let jwt_secret =
        std::env::var("JWT_SECRET").map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let exp = Utc::now().timestamp() as u64 + SESSION_DURATION_SECONDS;
    let claim = Claims {
        exp,
        email,
        device_id: None,
        device_name: None,
    };

    encode(
        &Header::default(),
        &claim,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    )
    .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn encode_device_jwt(
    email: String,
    device_id: String,
    device_name: String,
) -> Result<String, axum::http::StatusCode> {
    let jwt_secret =
        std::env::var("JWT_SECRET").map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let claim = Claims {
        exp: u64::MAX,
        email,
        device_id: Some(device_id),
        device_name: Some(device_name),
    };

    encode(
        &Header::default(),
        &claim,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    )
    .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn decode_jwt(jwt_token: &str) -> Result<TokenData<Claims>, axum::http::StatusCode> {
    let jwt_secret =
        std::env::var("JWT_SECRET").map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    decode(
        jwt_token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &Validation::default(),
    )
    .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn hash(input: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(input, bcrypt::DEFAULT_COST)
}

pub fn build_cookie<'a>(key: &str, token: String) -> Cookie<'a> {
    Cookie::build((key.to_string(), token))
        .path("/")
        .http_only(true)
        .max_age(cookie::time::Duration::seconds(
            SESSION_DURATION_SECONDS.try_into().unwrap(),
        ))
        .secure(!cfg!(debug_assertions))
        .build()
}

pub fn extract_token<'a>(
    cookie_jar: &'a axum_extra::extract::cookie::CookieJar,
    headers: &'a HeaderMap,
) -> Option<&'a str> {
    cookie_jar.get("jwt").map(|c| c.value()).or_else(|| {
        headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
    })
}

use axum::http::HeaderMap;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::cookie::CookieJar;
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::Level;

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

fn read_jwt_secret() -> Result<String, AppError> {
    std::env::var("JWT_SECRET").map_err(|_| AppError::ConfigError("JWT_SECRET is not set".into()))
}

#[tracing::instrument(err(level = Level::ERROR))]
pub fn encode_jwt(email: String) -> Result<String, AppError> {
    let jwt_secret = read_jwt_secret()?;
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
    .map_err(|error| AppError::ApplicationError(error.to_string()))
}

#[tracing::instrument(err(level = Level::ERROR))]
pub fn encode_device_jwt(email: String, device_id: String, device_name: String) -> Result<String, AppError> {
    let jwt_secret = read_jwt_secret()?;
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
    .map_err(|error| AppError::ApplicationError(error.to_string()))
}

#[tracing::instrument(err(level = Level::ERROR))]
pub fn decode_jwt(jwt_token: &str) -> Result<TokenData<Claims>, AppError> {
    let jwt_secret = read_jwt_secret()?;
    decode(
        jwt_token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &Validation::default(),
    )
    .map_err(|error| AppError::ApplicationError(error.to_string()))
}

#[tracing::instrument(err(level = Level::ERROR))]
pub fn hash(input: &str) -> Result<String, AppError> {
    bcrypt::hash(input, bcrypt::DEFAULT_COST).map_err(|error| AppError::ApplicationError(error.to_string()))
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

pub async fn authenticate_cookie(pool: &PgPool, cookie_jar: &CookieJar) -> Result<User, AppError> {
    let jwt = cookie_jar.get("jwt").ok_or(AppError::Unauthorized)?.value();
    authenticate_token(pool, jwt).await
}

pub async fn authenticate_request(
    pool: &PgPool,
    cookie_jar: &CookieJar,
    headers: &HeaderMap,
) -> Result<User, AppError> {
    let token = extract_token(cookie_jar, headers).ok_or(AppError::Unauthorized)?;
    authenticate_token(pool, token).await
}

#[tracing::instrument(skip(pool), err(level = Level::ERROR))]
pub async fn authenticate_token(pool: &PgPool, token: &str) -> Result<User, AppError> {
    let token_data = decode_jwt(token).map_err(|_| AppError::Unauthorized)?;
    match sqlx::query_as::<_, User>("SELECT id, email, timezone FROM users WHERE email = $1")
        .bind(token_data.claims.email)
        .fetch_one(pool)
        .await
    {
        Ok(user) => Ok(user),
        Err(sqlx::Error::RowNotFound) => Err(AppError::Unauthorized),
        Err(error) => Err(error.into()),
    }
}

pub fn extract_token<'a>(cookie_jar: &'a CookieJar, headers: &'a HeaderMap) -> Option<&'a str> {
    cookie_jar.get("jwt").map(|c| c.value()).or_else(|| {
        headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
    })
}

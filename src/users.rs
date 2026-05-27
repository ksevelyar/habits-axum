use axum::response::IntoResponse;
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use bcrypt::verify;
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::AppError;

#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    pub exp: usize,
    pub email: String,
}

#[derive(Deserialize)]
pub struct CreateSessionPayload {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct CreateUserPayload {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct BackendUser {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
}

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct User {
    pub id: i64,
    pub email: String,
}

pub async fn current(
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
) -> Result<Json<User>, AppError> {
    let user = authenticate_user(&pool, &cookie_jar).await?;
    Ok(Json(user))
}

pub async fn create_session(
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
    Json(user_data): Json<CreateSessionPayload>,
) -> impl IntoResponse {
    let user = find_user(&pool, &user_data.email)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let authenticated = verify(&user_data.password, &user.password_hash).unwrap_or(false);

    if !authenticated {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let jwt_token = encode_jwt(user.email).map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok((
        StatusCode::CREATED,
        cookie_jar.add(build_cookie("jwt", jwt_token, 24)),
    ))
}

async fn find_user(pool: &PgPool, email: &str) -> Result<BackendUser, sqlx::Error> {
    sqlx::query_as::<_, BackendUser>("SELECT * FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
}

pub async fn authenticate_token(pool: &PgPool, token: &str) -> Result<User, AppError> {
    let token_data = decode_jwt(token.to_string()).map_err(|_| AppError::Unauthorized)?;
    sqlx::query_as::<_, User>("SELECT id, email FROM users WHERE email = $1")
        .bind(token_data.claims.email)
        .fetch_one(pool)
        .await
        .map_err(|_| AppError::Unauthorized)
}

pub async fn authenticate_user(pool: &PgPool, cookie_jar: &CookieJar) -> Result<User, AppError> {
    let jwt = cookie_jar.get("jwt").ok_or(AppError::Unauthorized)?.value();
    authenticate_token(pool, jwt).await
}

pub fn encode_jwt(email: String) -> Result<String, StatusCode> {
    let jwt_secret = std::env::var("JWT_SECRET").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let now = Utc::now();
    let expire: chrono::TimeDelta = Duration::hours(24 * 7);
    let exp: usize = (now + expire).timestamp() as usize;
    let claim = Claims { exp, email };

    encode(
        &Header::default(),
        &claim,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn decode_jwt(jwt_token: String) -> Result<TokenData<Claims>, StatusCode> {
    let jwt_secret = std::env::var("JWT_SECRET").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    decode(
        &jwt_token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn build_cookie<'a>(key: &str, token: String, duration_hrs: i64) -> Cookie<'a> {
    Cookie::build((key.to_string(), token))
        .path("/")
        .http_only(true)
        .max_age(cookie::time::Duration::hours(duration_hrs))
        .secure(!cfg!(debug_assertions))
        .build()
}

fn hash(input: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(input, bcrypt::DEFAULT_COST)
}

pub async fn create(
    State(pool): State<PgPool>,
    Json(user_data): Json<CreateUserPayload>,
) -> Result<(StatusCode, Json<User>), StatusCode> {
    let hashed_password =
        hash(&user_data.password).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = sqlx::query_as::<_, User>(
        "
        INSERT INTO users (email, password_hash)
        VALUES ($1, $2)
        RETURNING id, email
        ",
    )
    .bind(user_data.email)
    .bind(hashed_password)
    .fetch_one(&pool)
    .await
    .map_err(|err| {
        dbg!(err);
        StatusCode::BAD_REQUEST
    })?;

    Ok((StatusCode::CREATED, Json(user)))
}

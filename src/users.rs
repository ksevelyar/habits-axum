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
#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub exp: usize,
    pub email: String,
}

#[derive(Deserialize)]
pub struct SignInData {
    pub email: String,
    pub password: String,
}

pub async fn current(
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
) -> Result<Json<CurrentUser>, StatusCode> {
    let jwt = cookie_jar.get("jwt").ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = decode_jwt(jwt.to_string()).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user = find_user(&pool, &claims.claims.email)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(Json(user))
}

pub async fn create(
    State(pool): State<PgPool>,
    cookie_jar: CookieJar,
    Json(user_data): Json<SignInData>,
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

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct CurrentUser {
    pub email: String,
    pub password_hash: String,
}

async fn find_user(pool: &PgPool, email: &str) -> Result<CurrentUser, sqlx::Error> {
    sqlx::query_as::<_, CurrentUser>("SELECT * FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
}

pub fn encode_jwt(email: String) -> Result<String, StatusCode> {
    let jwt_secret = std::env::var("JWT_SECRET").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let now = Utc::now();
    let expire: chrono::TimeDelta = Duration::hours(24);
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

pub fn build_cookie<'a>(key: &str, token: String, duration_hrs: i64) -> Cookie<'a> {
    Cookie::build((key.to_string(), token))
        .path("/")
        .http_only(true)
        .max_age(cookie::time::Duration::hours(duration_hrs))
        .secure(!cfg!(debug_assertions))
        .build()
}

use bcrypt::{DEFAULT_COST, hash};

#[derive(Deserialize)]
pub struct RegisterData {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub email: String,
}

pub async fn register(
    State(pool): State<PgPool>,
    Json(user_data): Json<RegisterData>,
) -> Result<(StatusCode, Json<User>), StatusCode> {
    let hashed_password =
        hash(&user_data.password, DEFAULT_COST).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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

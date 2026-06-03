use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::env;

#[derive(Serialize, sqlx::FromRow, Debug, Clone)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub timezone: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct BackendUser {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
}

#[derive(Serialize)]
pub struct DeviceTokenResponse {
    pub device_id: String,
    pub device_name: String,
    pub token: String,
}

pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<BackendUser, sqlx::Error> {
    sqlx::query_as::<_, BackendUser>("SELECT * FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
}

pub async fn set_dev_password(pool: &PgPool) {
    if let Ok(dev_password) = env::var("DEV_PASSWORD") {
        let hash = crate::authentication::hash(&dev_password).unwrap();
        sqlx::query(
            "INSERT INTO users (handle, email, password_hash, inserted_at, updated_at)
             VALUES ($1, $2, $3, NOW(), NOW())
             ON CONFLICT (email) DO UPDATE SET password_hash = $3",
        )
        .bind("ksevelyar")
        .bind("ksevelyar@gmail.com")
        .bind(&hash)
        .execute(pool)
        .await
        .expect("Failed to seed dev user");
        println!("🐗 Seeded dev user: ksevelyar@gmail.com");
    }
}

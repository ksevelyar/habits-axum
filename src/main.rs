use habits_axum::{app, users};
use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() {
    console_subscriber::init();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");

    sqlx::migrate!().run(&pool).await.expect("Migrations failed");

    users::set_dev_password(&pool).await;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3003").await.unwrap();
    tracing::info!("🐗 Listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app(pool)).await.unwrap();
}

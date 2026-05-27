use habits_axum::app;
use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");

    tracing_subscriber::fmt::init();

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Migrations failed");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3003").await.unwrap();
    println!("🐗 Listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app(pool)).await.unwrap();
}

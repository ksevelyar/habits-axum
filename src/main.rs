mod users;

use axum::Router;
use axum::http::{HeaderValue, Method, header::CONTENT_TYPE};
use axum::routing::{get, post};
use sqlx::postgres::PgPoolOptions;
use std::env;

use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

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

    let cors = CorsLayer::new()
        .allow_origin("http://habits.lcl:3000".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE])
        .allow_credentials(true);

    let app = Router::new()
        .route("/sessions", post(users::create))
        .route("/sessions/current", get(users::current))
        .route("/users", post(users::register))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3003").await.unwrap();
    println!("🐗 Listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}

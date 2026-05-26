mod chains;
mod metrics;
mod tasks;
mod users;

use axum::Router;
use axum::http::{HeaderValue, Method, header::CONTENT_TYPE};
use axum::routing::{delete, get, patch, post};
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
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PATCH])
        .allow_headers([CONTENT_TYPE])
        .allow_credentials(true);

    let app = Router::new()
        .route("/sessions", post(users::create))
        .route("/sessions/current", get(users::current))
        .route("/users", post(users::register))
        .route("/chains", get(chains::list))
        .route("/chains", post(chains::create))
        .route("/chains/{chain_id}", patch(chains::update))
        .route("/chains/{chain_id}", delete(chains::delete))
        .route("/chains/{chain_id}", get(chains::show))
        .route("/tasks", get(tasks::list))
        .route("/tasks", post(tasks::create))
        .route("/tasks/{task_id}", patch(tasks::update))
        .route("/tasks/{task_id}", delete(tasks::delete))
        .route("/tasks/{task_id}", get(tasks::show))
        .route("/metrics", post(metrics::upsert))
        .route("/metrics", get(metrics::get_by_date))
        .route("/metrics_history", get(metrics::history))
        .route("/metrics/{metric_id}", delete(metrics::delete))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3003").await.unwrap();
    println!("🐗 Listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}

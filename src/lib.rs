pub mod chains;
pub mod error;
pub mod metrics;
pub mod notifications;
pub mod tasks;
pub mod users;

use axum::Router;
use axum::http::{HeaderValue, Method, header::CONTENT_TYPE};
use axum::routing::{delete, get, patch, post};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn app(pool: PgPool) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(
            std::env::var("ORIGIN")
                .unwrap()
                .parse::<HeaderValue>()
                .unwrap(),
        )
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PATCH])
        .allow_headers([CONTENT_TYPE])
        .allow_credentials(true);

    Router::new()
        .route("/sessions", post(users::create_session))
        .route("/sessions/current", get(users::current))
        .route("/users", post(users::create))
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
        .route("/websocket/notifications", get(notifications::connect))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(pool)
}

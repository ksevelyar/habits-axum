pub mod authentication;
pub mod chains;
pub mod error;
pub mod handlers;
pub mod metrics;
pub mod notifications;
pub mod tasks;
pub mod users;

use axum::Router;
use axum::http::{HeaderValue, Method, header::CONTENT_TYPE};
use axum::routing::{delete, get, patch, post};
use sqlx::PgPool;
use std::collections::HashMap;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct UserChannel {
    pub broadcast: broadcast::Sender<String>,
    pub scheduler: JoinHandle<()>,
}

#[derive(Debug)]
pub struct AppState {
    pub channels: RwLock<HashMap<i64, UserChannel>>,
    pub pool: PgPool,
}

pub fn app(pool: PgPool) -> Router {
    let state = Arc::new(AppState {
        channels: RwLock::new(HashMap::new()),
        pool,
    });

    let cors_origins: Vec<HeaderValue> = std::env::var("CORS_ORIGINS")
        .unwrap()
        .split(',')
        .map(|host| host.trim().parse().unwrap())
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(cors_origins)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PATCH])
        .allow_headers([CONTENT_TYPE])
        .allow_credentials(true);

    Router::new()
        .route("/sessions", post(handlers::users::create_session))
        .route("/sessions/current", get(handlers::users::current))
        .route("/users", post(handlers::users::create))
        .route("/devices", post(handlers::users::create_device))
        .route("/chains", get(handlers::chains::list))
        .route("/chains", post(handlers::chains::create))
        .route("/chains/{chain_id}", patch(handlers::chains::update))
        .route("/chains/{chain_id}", delete(handlers::chains::delete))
        .route("/chains/{chain_id}", get(handlers::chains::show))
        .route("/tasks", get(handlers::tasks::list))
        .route("/tasks", post(handlers::tasks::create))
        .route("/tasks/{task_id}", patch(handlers::tasks::update))
        .route("/tasks/{task_id}", delete(handlers::tasks::delete))
        .route("/tasks/{task_id}", get(handlers::tasks::show))
        .route("/metrics", post(handlers::metrics::upsert))
        .route("/metrics", get(handlers::metrics::get_by_date))
        .route("/metrics_history", get(handlers::metrics::history))
        .route("/metrics/{metric_id}", delete(handlers::metrics::delete))
        .route("/websocket/notifications", get(handlers::notifications::connect))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

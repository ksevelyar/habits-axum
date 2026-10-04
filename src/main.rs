use habits_axum::{build_app, telegram, users};
use jiff::Zoned;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::layer::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

struct LocalTime;

impl FormatTime for LocalTime {
    fn format_time(&self, writer: &mut Writer<'_>) -> std::fmt::Result {
        write!(writer, "{}", Zoned::now().strftime("%Y-%m-%d %H:%M:%S%.1f"))
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(console_subscriber::ConsoleLayer::builder().spawn())
        .with(
            tracing_subscriber::fmt::layer()
                .with_timer(LocalTime)
                .with_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))),
        )
        .init();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");

    sqlx::migrate!().run(&pool).await.expect("Migrations failed");

    users::set_dev_password(&pool).await;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3003").await.unwrap();
    tracing::info!("🐗 Listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, build_app(pool, telegram::build())).await.unwrap();
}

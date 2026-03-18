use anyhow::Result;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod config;
mod db;
mod errors;
mod http_server;
mod metrics;
mod models;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let config = config::load().expect("Failed to load configuration");

    info!(
        version = env!("CARGO_PKG_VERSION"),
        host = %config.host,
        port = config.port,
        "Starting application"
    );

    let db = db::connect(&config.database_url).await?;

    info!("Running database migrations");
    db.run_migrations().await?;

    http_server::run_server(config, db).await?;

    Ok(())
}

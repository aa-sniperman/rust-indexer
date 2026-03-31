use anyhow::Result;
use rust_indexer::{config::AppConfig, init_tracing, store::postgres::PostgresStore};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let config = AppConfig::from_env()?;
    init_tracing(&config.log_level);

    let postgres = PostgresStore::connect(&config.postgres_url).await?;
    postgres.run_migrations().await?;

    info!("database migrations completed");
    Ok(())
}

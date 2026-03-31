use anyhow::Result;
use rust_indexer::{
    config::AppConfig,
    indexer::shreds::run_realtime_indexer,
    init_tracing,
    store::{postgres::PostgresStore, redis::RedisStore, repository::Repository},
};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let config = AppConfig::from_env()?;
    init_tracing(&config.log_level);

    let postgres = PostgresStore::connect(&config.postgres_url).await?;
    postgres.run_migrations().await?;

    let redis = RedisStore::connect(&config.redis_url, config.redis_ttl_secs)?;
    redis.ping().await?;

    let repository = Repository::new(postgres, redis);

    info!(
        ws_url = %config.rise_ws_rpc_url,
        "starting realtime indexer"
    );

    run_realtime_indexer(&config.rise_ws_rpc_url, repository).await?;
    Ok(())
}

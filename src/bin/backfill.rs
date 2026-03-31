use anyhow::Result;
use rust_indexer::{
    config::AppConfig,
    indexer::backfill::run_backfill,
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
        backfill_start_block = config.backfill_start_block,
        backfill_batch_size = config.backfill_batch_size,
        http_url = %config.rise_http_rpc_url,
        "starting backfill worker"
    );

    run_backfill(
        &config.rise_http_rpc_url,
        config.backfill_start_block,
        repository,
    )
    .await?;

    Ok(())
}

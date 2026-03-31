use std::net::SocketAddr;

use anyhow::Result;
use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use rust_indexer::{
    config::AppConfig,
    init_tracing,
    store::{postgres::PostgresStore, redis::RedisStore, repository::Repository},
};
use serde_json::{Value, json};
use tracing::info;

#[derive(Clone)]
struct AppState {
    repository: Repository,
}

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
    let state = AppState { repository };

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/", get(root))
        .with_state(state);

    let addr: SocketAddr = config.server_bind_addr;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!(%addr, "rpc server scaffold is listening");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn root(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let _repository = state.repository;
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "status": "not_implemented",
            "message": "JSON-RPC methods will be added in the next milestone"
        })),
    )
}

async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

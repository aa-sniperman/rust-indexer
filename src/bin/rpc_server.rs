use std::net::SocketAddr;

use anyhow::Result;
use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use rust_indexer::{
    config::AppConfig,
    init_tracing,
    models::JsonRpcRequest,
    rpc::http::RiseHttpClient,
    server::jsonrpc::JsonRpcService,
    store::{postgres::PostgresStore, redis::RedisStore, repository::Repository},
};
use serde_json::{Value, json};
use tracing::info;

#[derive(Clone)]
struct AppState {
    service: JsonRpcService,
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
    let upstream = RiseHttpClient::new(&config.rise_http_rpc_url)?;
    let service = JsonRpcService::new(repository, upstream);
    let state = AppState { service };

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/", post(root))
        .with_state(state);

    let addr: SocketAddr = config.server_bind_addr;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!(%addr, "rpc server scaffold is listening");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn root(State(state): State<AppState>, Json(request): Json<JsonRpcRequest>) -> Json<Value> {
    Json(state.service.handle(request).await)
}

async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

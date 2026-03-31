use std::{env, net::SocketAddr};

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub rise_http_rpc_url: String,
    pub rise_ws_rpc_url: String,
    pub postgres_url: String,
    pub redis_url: String,
    pub server_bind_addr: SocketAddr,
    pub backfill_start_block: u64,
    pub backfill_batch_size: u64,
    pub redis_ttl_secs: u64,
    pub log_level: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let rise_http_rpc_url = required_env("RISE_HTTP_RPC_URL")?;
        let rise_ws_rpc_url = required_env("RISE_WS_RPC_URL")?;
        let postgres_url = required_env("POSTGRES_URL")?;
        let redis_url = required_env("REDIS_URL")?;
        let server_bind_addr = required_env("SERVER_BIND_ADDR")?
            .parse()
            .context("SERVER_BIND_ADDR must be a valid socket address")?;
        let backfill_start_block = parse_u64_env("BACKFILL_START_BLOCK")?;
        let backfill_batch_size = parse_u64_env("BACKFILL_BATCH_SIZE")?;
        let redis_ttl_secs = parse_u64_env("REDIS_TTL_SECS")?;
        let log_level = required_env("LOG_LEVEL")?;

        Ok(Self {
            rise_http_rpc_url,
            rise_ws_rpc_url,
            postgres_url,
            redis_url,
            server_bind_addr,
            backfill_start_block,
            backfill_batch_size,
            redis_ttl_secs,
            log_level,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    env::var(name).with_context(|| format!("missing required env var {name}"))
}

fn parse_u64_env(name: &str) -> Result<u64> {
    required_env(name)?
        .parse()
        .with_context(|| format!("{name} must be a valid u64"))
}

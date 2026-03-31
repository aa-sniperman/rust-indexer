use anyhow::Result;
use redis::{AsyncCommands, aio::MultiplexedConnection};
use tokio::sync::OnceCell;

use crate::models::ShredTxRecord;

pub struct RedisStore {
    client: redis::Client,
    ttl_secs: u64,
    connection: OnceCell<MultiplexedConnection>,
}

impl Clone for RedisStore {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            ttl_secs: self.ttl_secs,
            connection: OnceCell::new(),
        }
    }
}

impl RedisStore {
    pub fn new(client: redis::Client, ttl_secs: u64) -> Self {
        Self {
            client,
            ttl_secs,
            connection: OnceCell::new(),
        }
    }

    pub fn connect(redis_url: &str, ttl_secs: u64) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self::new(client, ttl_secs))
    }

    pub async fn ping(&self) -> Result<()> {
        let mut conn = self.connection().await?;
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(())
    }

    pub async fn set_shred_tx(&self, record: &ShredTxRecord) -> Result<()> {
        let mut conn = self.connection().await?;
        let payload = serde_json::to_string(record)?;

        if self.ttl_secs == 0 {
            let _: () = conn.set(shred_tx_key(&record.tx_hash), payload).await?;
        } else {
            let ttl = self.ttl_secs.try_into()?;
            let _: () = conn
                .set_ex(shred_tx_key(&record.tx_hash), payload, ttl)
                .await?;
        }

        Ok(())
    }

    pub async fn get_shred_tx(&self, tx_hash: &str) -> Result<Option<ShredTxRecord>> {
        let mut conn = self.connection().await?;
        let value: Option<String> = conn.get(shred_tx_key(tx_hash)).await?;

        value
            .map(|payload| serde_json::from_str(&payload).map_err(Into::into))
            .transpose()
    }

    pub async fn flush_all(&self) -> Result<()> {
        let mut conn = self.connection().await?;
        let _: String = redis::cmd("FLUSHALL").query_async(&mut conn).await?;
        Ok(())
    }

    async fn connection(&self) -> Result<MultiplexedConnection> {
        let conn = self
            .connection
            .get_or_try_init(|| async {
                self.client
                    .get_multiplexed_async_connection()
                    .await
                    .map_err(anyhow::Error::from)
            })
            .await?;

        Ok(conn.clone())
    }
}

fn shred_tx_key(tx_hash: &str) -> String {
    format!("shred_tx:{tx_hash}")
}

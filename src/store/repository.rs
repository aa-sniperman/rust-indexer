use std::collections::HashSet;

use anyhow::Result;

use crate::{
    models::ShredTxRecord,
    store::{postgres::PostgresStore, redis::RedisStore},
};

#[derive(Clone)]
pub struct Repository {
    postgres: PostgresStore,
    redis: RedisStore,
}

impl Repository {
    pub fn new(postgres: PostgresStore, redis: RedisStore) -> Self {
        Self { postgres, redis }
    }

    pub fn postgres(&self) -> &PostgresStore {
        &self.postgres
    }

    pub fn redis(&self) -> &RedisStore {
        &self.redis
    }

    pub async fn cache_shred_tx(&self, record: &ShredTxRecord) -> Result<()> {
        self.redis.set_shred_tx(record).await?;
        Ok(())
    }

    pub async fn persist_shred_tx(&self, record: &ShredTxRecord) -> Result<()> {
        self.postgres.upsert_shred_tx(record).await?;
        Ok(())
    }

    pub async fn get_shred_tx(&self, tx_hash: &str) -> Result<Option<ShredTxRecord>> {
        if let Some(record) = self.redis.get_shred_tx(tx_hash).await? {
            return Ok(Some(record));
        }

        if let Some(record) = self.postgres.get_shred_tx(tx_hash).await? {
            self.redis.set_shred_tx(&record).await?;
            return Ok(Some(record));
        }

        Ok(None)
    }

    pub async fn get_existing_tx_hashes(&self, tx_hashes: &[String]) -> Result<HashSet<String>> {
        self.postgres.get_existing_tx_hashes(tx_hashes).await
    }

    pub async fn get_backfill_resume_block(&self, job_name: &str, from_block: u64) -> Result<u64> {
        let progress = self.postgres.get_backfill_progress(job_name).await?;
        Ok(progress
            .map(|progress| (progress.last_completed_block + 1).max(from_block as i64) as u64)
            .unwrap_or(from_block))
    }

    pub async fn save_backfill_progress(
        &self,
        job_name: &str,
        last_completed_block: i64,
    ) -> Result<()> {
        self.postgres
            .save_backfill_progress(job_name, last_completed_block)
            .await
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::{
        models::ShredTxRecord,
        store::{postgres::PostgresStore, redis::RedisStore},
    };

    use super::Repository;

    #[tokio::test]
    async fn store_round_trip_smoke_test() -> Result<()> {
        if std::env::var("RUN_STORE_TESTS").ok().as_deref() != Some("1") {
            eprintln!("Skipping store smoke test. Set RUN_STORE_TESTS=1 to enable.");
            return Ok(());
        }

        let postgres_url = std::env::var("POSTGRES_URL")?;
        let redis_url = std::env::var("REDIS_URL")?;

        let postgres = PostgresStore::connect(&postgres_url).await?;
        postgres.run_migrations().await?;
        postgres.truncate_all().await?;

        let redis = RedisStore::connect(&redis_url, 300)?;
        redis.flush_all().await?;

        let repository = Repository::new(postgres, redis);
        let record = sample_shred_tx();

        repository.cache_shred_tx(&record).await?;
        repository.persist_shred_tx(&record).await?;

        let stored_record = repository.get_shred_tx(&record.tx_hash).await?;
        assert_eq!(stored_record, Some(record));

        Ok(())
    }

    fn sample_shred_tx() -> ShredTxRecord {
        ShredTxRecord {
            tx_hash: "0xtesthash".to_string(),
            block_number: Some(42),
            block_timestamp: Some(1700000000),
            block_hash: Some("0xblockhash".to_string()),
            shred_idx: Some(1),
            tx_offset_in_shred: 0,
            starting_log_index: Some(7),
            signer: Some("0xfrom".to_string()),
            to_address: Some("0xto".to_string()),
            tx_type: Some("0x2".to_string()),
            receipt_status: Some("0x1".to_string()),
            transaction_json: r#"{"hash":"0xtesthash"}"#.to_string(),
            receipt_json: r#"{"transactionHash":"0xtesthash"}"#.to_string(),
            state_changes_json: Some(r#"{"0xabc":{"nonce":1}}"#.to_string()),
            source: "test".to_string(),
        }
    }
}

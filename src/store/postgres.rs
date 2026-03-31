use std::collections::HashSet;

use anyhow::Result;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};

use crate::models::{BackfillProgress, ShredTxRecord};

#[derive(Clone)]
pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    pub async fn upsert_shred_tx(&self, record: &ShredTxRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO shred_transactions (
                tx_hash,
                block_number,
                block_timestamp,
                block_hash,
                shred_idx,
                tx_offset_in_shred,
                starting_log_index,
                signer,
                to_address,
                tx_type,
                receipt_status,
                transaction_json,
                receipt_json,
                state_changes_json,
                source
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            ON CONFLICT (tx_hash) DO UPDATE
            SET
                block_number = EXCLUDED.block_number,
                block_timestamp = EXCLUDED.block_timestamp,
                block_hash = EXCLUDED.block_hash,
                shred_idx = EXCLUDED.shred_idx,
                tx_offset_in_shred = EXCLUDED.tx_offset_in_shred,
                starting_log_index = EXCLUDED.starting_log_index,
                signer = EXCLUDED.signer,
                to_address = EXCLUDED.to_address,
                tx_type = EXCLUDED.tx_type,
                receipt_status = EXCLUDED.receipt_status,
                transaction_json = EXCLUDED.transaction_json,
                receipt_json = EXCLUDED.receipt_json,
                state_changes_json = EXCLUDED.state_changes_json,
                source = EXCLUDED.source
            "#,
        )
        .bind(&record.tx_hash)
        .bind(record.block_number)
        .bind(record.block_timestamp)
        .bind(&record.block_hash)
        .bind(record.shred_idx)
        .bind(record.tx_offset_in_shred)
        .bind(record.starting_log_index)
        .bind(&record.signer)
        .bind(&record.to_address)
        .bind(&record.tx_type)
        .bind(&record.receipt_status)
        .bind(&record.transaction_json)
        .bind(&record.receipt_json)
        .bind(&record.state_changes_json)
        .bind(&record.source)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_shred_tx(&self, tx_hash: &str) -> Result<Option<ShredTxRecord>> {
        let row = sqlx::query(
            r#"
            SELECT
                tx_hash,
                block_number,
                block_timestamp,
                block_hash,
                shred_idx,
                tx_offset_in_shred,
                starting_log_index,
                signer,
                to_address,
                tx_type,
                receipt_status,
                transaction_json,
                receipt_json,
                state_changes_json,
                source
            FROM shred_transactions
            WHERE tx_hash = $1
            "#,
        )
        .bind(tx_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(map_shred_tx_row))
    }

    pub async fn get_existing_tx_hashes(&self, tx_hashes: &[String]) -> Result<HashSet<String>> {
        if tx_hashes.is_empty() {
            return Ok(HashSet::new());
        }

        let rows = sqlx::query(
            r#"
            SELECT tx_hash
            FROM shred_transactions
            WHERE tx_hash = ANY($1)
            "#,
        )
        .bind(tx_hashes)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| row.get::<String, _>("tx_hash"))
            .collect())
    }

    pub async fn truncate_all(&self) -> Result<()> {
        sqlx::query("TRUNCATE TABLE backfill_progress, shred_transactions")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_backfill_progress(&self, job_name: &str) -> Result<Option<BackfillProgress>> {
        let row = sqlx::query(
            r#"
            SELECT job_name, last_completed_block
            FROM backfill_progress
            WHERE job_name = $1
            "#,
        )
        .bind(job_name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| BackfillProgress {
            job_name: row.get("job_name"),
            last_completed_block: row.get("last_completed_block"),
        }))
    }

    pub async fn save_backfill_progress(
        &self,
        job_name: &str,
        last_completed_block: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO backfill_progress (job_name, last_completed_block)
            VALUES ($1, $2)
            ON CONFLICT (job_name) DO UPDATE
            SET
                last_completed_block = EXCLUDED.last_completed_block,
                updated_at = NOW()
            "#,
        )
        .bind(job_name)
        .bind(last_completed_block)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

fn map_shred_tx_row(row: sqlx::postgres::PgRow) -> ShredTxRecord {
    ShredTxRecord {
        tx_hash: row.get("tx_hash"),
        block_number: row.get("block_number"),
        block_timestamp: row.get("block_timestamp"),
        block_hash: row.get("block_hash"),
        shred_idx: row.get("shred_idx"),
        tx_offset_in_shred: row.get("tx_offset_in_shred"),
        starting_log_index: row.get("starting_log_index"),
        signer: row.get("signer"),
        to_address: row.get("to_address"),
        tx_type: row.get("tx_type"),
        receipt_status: row.get("receipt_status"),
        transaction_json: row.get("transaction_json"),
        receipt_json: row.get("receipt_json"),
        state_changes_json: row.get("state_changes_json"),
        source: row.get("source"),
    }
}

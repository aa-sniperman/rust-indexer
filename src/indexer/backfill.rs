use std::time::Duration;

use anyhow::Result;
use tokio::{sync::mpsc, time::sleep};
use tracing::{debug, error, info, warn};

use crate::{models::ShredTxRecord, rpc::http::RiseHttpClient, store::repository::Repository};

const CHANNEL_CAPACITY: usize = 1024;
const DURABILITY_RETRY_DELAY_SECS: u64 = 2;
const BACKFILL_JOB_NAME: &str = "backfill";
const LATEST_POLL_INTERVAL_SECS: u64 = 10;

pub async fn run_backfill(rpc_url: &str, start_block: u64, repository: Repository) -> Result<()> {
    let client = RiseHttpClient::new(rpc_url)?;
    let (cache_tx, cache_rx) = mpsc::channel(CHANNEL_CAPACITY);
    let (durable_tx, durable_rx) = mpsc::channel(CHANNEL_CAPACITY);

    let _cache_task = tokio::spawn(run_cache_worker(repository.clone(), cache_rx));
    let _durable_task = tokio::spawn(run_durable_worker(repository.clone(), durable_rx));

    let mut next_block = repository
        .get_backfill_resume_block(BACKFILL_JOB_NAME, start_block)
        .await?;
    info!(
        job_name = BACKFILL_JOB_NAME,
        next_block, "starting backfill"
    );

    loop {
        let latest_block = client.get_latest_block_number().await?;

        if next_block > latest_block {
            info!(
                job_name = BACKFILL_JOB_NAME,
                next_block,
                latest_block,
                sleep_secs = LATEST_POLL_INTERVAL_SECS,
                "backfill caught up, waiting for new blocks"
            );
            sleep(Duration::from_secs(LATEST_POLL_INTERVAL_SECS)).await;
            continue;
        }

        for block_number in next_block..=latest_block {
            let maybe_block = client.get_block_by_number(block_number).await?;
            let Some(block) = maybe_block else {
                warn!(block_number, "upstream returned null block during backfill");
                continue;
            };

            let tx_count = block.transactions.len();
            let tx_hashes: Vec<String> = block
                .transactions
                .iter()
                .map(|transaction| transaction.hash.clone())
                .collect();
            let existing_tx_hashes = repository.get_existing_tx_hashes(&tx_hashes).await?;
            let mut skipped_existing = 0usize;
            let mut enqueued_missing = 0usize;

            for transaction in block.transactions.iter().cloned() {
                let tx_hash = transaction.hash.clone();

                if existing_tx_hashes.contains(&tx_hash) {
                    skipped_existing += 1;
                    debug!(block_number, %tx_hash, "skipping backfill for tx already present locally");
                    continue;
                }

                let receipt = match client.get_transaction_receipt(&tx_hash).await? {
                    Some(receipt) => receipt,
                    None => {
                        warn!(%tx_hash, block_number, "missing receipt during backfill");
                        continue;
                    }
                };

                let record = ShredTxRecord::from_backfill(&block, transaction, receipt, "backfill");

                if let Err(error) = cache_tx.send(record.clone()).await {
                    error!(%tx_hash, %error, "failed to enqueue backfill cache write");
                }

                if let Err(error) = durable_tx.send(record).await {
                    error!(%tx_hash, %error, "failed to enqueue backfill durable write");
                }

                enqueued_missing += 1;
            }

            repository
                .save_backfill_progress(BACKFILL_JOB_NAME, block_number as i64)
                .await?;

            next_block = block_number + 1;

            info!(
                block_number,
                tx_count, skipped_existing, enqueued_missing, "completed backfill block"
            );
        }
    }
}

async fn run_cache_worker(
    repository: Repository,
    mut receiver: mpsc::Receiver<ShredTxRecord>,
) -> Result<()> {
    while let Some(record) = receiver.recv().await {
        let tx_hash = record.tx_hash.clone();
        if let Err(error) = repository.cache_shred_tx(&record).await {
            error!(%tx_hash, %error, "failed to cache backfill record");
        } else {
            debug!(%tx_hash, "cached backfill record");
        }
    }

    Ok(())
}

async fn run_durable_worker(
    repository: Repository,
    mut receiver: mpsc::Receiver<ShredTxRecord>,
) -> Result<()> {
    while let Some(record) = receiver.recv().await {
        let tx_hash = record.tx_hash.clone();
        let mut attempts = 0u32;

        loop {
            attempts += 1;
            match repository.persist_shred_tx(&record).await {
                Ok(()) => {
                    debug!(%tx_hash, attempts, "persisted backfill record");
                    break;
                }
                Err(error) => {
                    warn!(
                        %tx_hash,
                        attempts,
                        %error,
                        delay_secs = DURABILITY_RETRY_DELAY_SECS,
                        "failed to persist backfill record, retrying"
                    );
                    sleep(Duration::from_secs(DURABILITY_RETRY_DELAY_SECS)).await;
                }
            }
        }
    }

    Ok(())
}

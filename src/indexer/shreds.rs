use std::time::Duration;

use anyhow::Result;
use tokio::{
    sync::mpsc,
    time::{sleep, timeout},
};
use tracing::{debug, error, info, warn};

use crate::{
    models::{JsonRpcSubscriptionMessage, ShredTxRecord},
    rpc::ws::ShredWsClient,
    store::repository::Repository,
};

const CHANNEL_CAPACITY: usize = 1024;
const RECEIVE_TIMEOUT_SECS: u64 = 30;
const DURABILITY_RETRY_DELAY_SECS: u64 = 2;

pub async fn run_realtime_indexer(ws_url: &str, repository: Repository) -> Result<()> {
    let (cache_tx, cache_rx) = mpsc::channel(CHANNEL_CAPACITY);
    let (durable_tx, durable_rx) = mpsc::channel(CHANNEL_CAPACITY);

    let cache_task = tokio::spawn(run_cache_worker(repository.clone(), cache_rx));
    let durable_task = tokio::spawn(run_durable_worker(repository, durable_rx));
    let listen_task = tokio::spawn(run_shred_listener(ws_url.to_owned(), cache_tx, durable_tx));

    tokio::select! {
        result = cache_task => {
            result??;
        }
        result = durable_task => {
            result??;
        }
        result = listen_task => {
            result??;
        }
        result = tokio::signal::ctrl_c() => {
            result?;
            info!("received shutdown signal");
        }
    }

    Ok(())
}

async fn run_shred_listener(
    ws_url: String,
    cache_sender: mpsc::Sender<ShredTxRecord>,
    durable_sender: mpsc::Sender<ShredTxRecord>,
) -> Result<()> {
    let client = ShredWsClient::new(ws_url);
    let mut reconnect_delay = Duration::from_secs(1);

    loop {
        match run_subscription_loop(&client, &cache_sender, &durable_sender).await {
            Ok(()) => {
                warn!("shred subscription loop exited cleanly, reconnecting");
            }
            Err(error) => {
                warn!(error = %error, delay_secs = reconnect_delay.as_secs(), "shred subscription failed");
            }
        }

        sleep(reconnect_delay).await;
        reconnect_delay = (reconnect_delay * 2).min(Duration::from_secs(15));
    }
}

async fn run_subscription_loop(
    client: &ShredWsClient,
    cache_sender: &mpsc::Sender<ShredTxRecord>,
    durable_sender: &mpsc::Sender<ShredTxRecord>,
) -> Result<()> {
    let mut subscription = client.connect_and_subscribe().await?;
    let mut seen_messages = 0u64;

    loop {
        let payload = timeout(
            Duration::from_secs(RECEIVE_TIMEOUT_SECS),
            subscription.next_json_message(),
        )
        .await??;

        // warn!(
        //     subscription_id = subscription.subscription_id(),
        //     payload = %payload,
        //     "received raw websocket payload"
        // );

        let message: JsonRpcSubscriptionMessage = match serde_json::from_value(payload) {
            Ok(message) => message,
            Err(error) => {
                debug!(error = %error, "ignoring websocket payload that is not a shred notification");
                continue;
            }
        };

        if message.method != "eth_subscription" {
            debug!(method = %message.method, "ignoring unexpected websocket method");
            continue;
        }

        let result = message.params.result;
        let block_number = result.block_number;
        let block_timestamp = result.block_timestamp;
        let shred_idx = result.shred_idx;
        let starting_log_index = result.starting_log_index;
        let tx_count = result.transactions.len();
        let state_changes = result.state_changes;

        for (tx_offset_in_shred, envelope) in result.transactions.into_iter().enumerate() {
            if let Some(record) = ShredTxRecord::from_shred_envelope(
                envelope,
                block_number,
                block_timestamp,
                shred_idx,
                starting_log_index,
                tx_offset_in_shred as i32,
                &state_changes,
                "realtime",
            ) {
                let tx_hash = record.tx_hash.clone();
                let cache_result = cache_sender.send(record.clone()).await;
                let durable_result = durable_sender.send(record).await;

                if let Err(error) = cache_result {
                    warn!(%tx_hash, %error, "failed to enqueue realtime cache write");
                }

                if let Err(error) = durable_result {
                    warn!(%tx_hash, %error, "failed to enqueue realtime durable write");
                }
            } else {
                debug!("skipping shred transaction missing hash");
            }
        }

        seen_messages += 1;
        if seen_messages % 50 == 0 {
            info!(
                seen_messages,
                block_number,
                shred_idx,
                tx_count,
                subscription_id = subscription.subscription_id(),
                "processed realtime shred notifications"
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
            error!(%tx_hash, %error, "failed to cache realtime shred transaction");
            continue;
        }

        debug!(%tx_hash, "cached realtime shred transaction");
    }

    anyhow::bail!("realtime cache worker channel closed")
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
                    debug!(%tx_hash, attempts, "persisted realtime shred transaction to postgres");
                    break;
                }
                Err(error) => {
                    warn!(
                        %tx_hash,
                        attempts,
                        %error,
                        delay_secs = DURABILITY_RETRY_DELAY_SECS,
                        "failed to persist realtime data to postgres, retrying"
                    );
                    sleep(Duration::from_secs(DURABILITY_RETRY_DELAY_SECS)).await;
                }
            }
        }
    }

    anyhow::bail!("realtime durability worker channel closed")
}

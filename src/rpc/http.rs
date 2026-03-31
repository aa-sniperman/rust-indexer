use anyhow::{Context, Result};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::models::{RpcBlock, RpcReceipt};

#[derive(Clone)]
pub struct RiseHttpClient {
    client: Client,
    url: String,
}

impl RiseHttpClient {
    pub fn new(url: impl Into<String>) -> Result<Self> {
        let client = Client::builder().build()?;
        Ok(Self {
            client,
            url: url.into(),
        })
    }

    pub async fn get_block_by_number(&self, block_number: u64) -> Result<Option<RpcBlock>> {
        let hex_block = format!("0x{block_number:x}");
        self.rpc_call("eth_getBlockByNumber", json!([hex_block, true]))
            .await
    }

    pub async fn get_transaction_receipt(&self, tx_hash: &str) -> Result<Option<RpcReceipt>> {
        self.rpc_call("eth_getTransactionReceipt", json!([tx_hash]))
            .await
    }

    pub async fn get_transaction_by_hash(&self, tx_hash: &str) -> Result<Option<Value>> {
        self.rpc_call("eth_getTransactionByHash", json!([tx_hash]))
            .await
    }

    pub async fn get_latest_block_number(&self) -> Result<u64> {
        let result: Option<String> = self.rpc_call("eth_blockNumber", json!([])).await?;
        let latest = result.context("upstream returned null for eth_blockNumber")?;
        parse_u64_from_rpc_hex(&latest)
            .with_context(|| format!("invalid eth_blockNumber response: {latest}"))
    }

    pub async fn send_jsonrpc(&self, payload: &Value) -> Result<Value> {
        let response = self
            .client
            .post(&self.url)
            .json(payload)
            .send()
            .await
            .context("failed to call upstream json-rpc")?;

        response
            .json()
            .await
            .context("invalid upstream json-rpc response")
    }

    async fn rpc_call<T>(&self, method: &str, params: Value) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let response = self
            .client
            .post(&self.url)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params
            }))
            .send()
            .await
            .with_context(|| format!("failed to call upstream method {method}"))?;

        let value: Value = response
            .json()
            .await
            .with_context(|| format!("invalid upstream response for method {method}"))?;

        if let Some(error) = value.get("error") {
            anyhow::bail!("upstream rpc error for {method}: {error}");
        }

        value
            .get("result")
            .cloned()
            .map(|result| serde_json::from_value(result).map_err(Into::into))
            .transpose()
    }
}

fn parse_u64_from_rpc_hex(input: &str) -> Option<u64> {
    if let Some(hex) = input.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).ok()
    } else {
        input.parse::<u64>().ok()
    }
}

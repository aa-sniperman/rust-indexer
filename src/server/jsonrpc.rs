use anyhow::{Context, Result};
use serde_json::{Value, json};
use tracing::info;

use crate::{
    models::{
        JsonRpcErrorObject, JsonRpcErrorResponse, JsonRpcRequest, JsonRpcSuccessResponse,
        ShredTxRecord, parse_receipt_value, parse_transaction_value,
    },
    rpc::http::RiseHttpClient,
    store::repository::Repository,
};

#[derive(Clone)]
pub struct JsonRpcService {
    repository: Repository,
    upstream: RiseHttpClient,
}

#[derive(Clone, Copy)]
enum HitSource {
    Redis,
    Postgres,
    UpstreamFill,
}

impl JsonRpcService {
    pub fn new(repository: Repository, upstream: RiseHttpClient) -> Self {
        Self {
            repository,
            upstream,
        }
    }

    pub async fn handle(&self, request: JsonRpcRequest) -> Value {
        let id = request.id.clone();

        if request.jsonrpc != "2.0" {
            return error_response(id, -32600, "invalid json-rpc version");
        }

        match request.method.as_str() {
            "eth_getTransactionByHash" => match first_param_as_str(&request.params) {
                Some(tx_hash) => match self.get_transaction_by_hash(&tx_hash).await {
                    Ok((result, source)) => {
                        info!(%tx_hash, source = hit_source_name(source), "served eth_getTransactionByHash");
                        success_response(id, result)
                    }
                    Err(error) => error_response(id, -32000, &format!("internal error: {error}")),
                },
                None => error_response(id, -32602, "missing transaction hash"),
            },
            "eth_getTransactionReceipt" => match first_param_as_str(&request.params) {
                Some(tx_hash) => match self.get_transaction_receipt(&tx_hash).await {
                    Ok((result, source)) => {
                        info!(%tx_hash, source = hit_source_name(source), "served eth_getTransactionReceipt");
                        success_response(id, result)
                    }
                    Err(error) => error_response(id, -32000, &format!("internal error: {error}")),
                },
                None => error_response(id, -32602, "missing transaction hash"),
            },
            _ => match self
                .upstream
                .send_jsonrpc(&json!({
                    "jsonrpc": request.jsonrpc,
                    "id": request.id,
                    "method": request.method,
                    "params": request.params
                }))
                .await
            {
                Ok(response) => {
                    info!("served passthrough json-rpc method");
                    response
                }
                Err(error) => error_response(id, -32000, &format!("upstream error: {error}")),
            },
        }
    }

    async fn get_transaction_by_hash(&self, tx_hash: &str) -> Result<(Value, HitSource)> {
        if let Some(record) = self.repository.redis().get_shred_tx(tx_hash).await? {
            return Ok((parse_transaction_value(&record)?, HitSource::Redis));
        }

        if let Some(record) = self.repository.postgres().get_shred_tx(tx_hash).await? {
            self.repository.cache_shred_tx(&record).await?;
            return Ok((parse_transaction_value(&record)?, HitSource::Postgres));
        }

        let transaction = self.upstream.get_transaction_by_hash(tx_hash).await?;
        let Some(transaction_value) = transaction else {
            return Ok((Value::Null, HitSource::UpstreamFill));
        };

        let receipt = self.upstream.get_transaction_receipt(tx_hash).await?;
        if let Some(receipt_value) = receipt_to_value(receipt)? {
            if let Some(record) = ShredTxRecord::from_rpc_values(
                transaction_value.clone(),
                Some(receipt_value),
                "upstream_fill",
            ) {
                self.repository.persist_shred_tx(&record).await?;
                self.repository.cache_shred_tx(&record).await?;
            }
        } else if let Some(record) =
            ShredTxRecord::from_rpc_values(transaction_value.clone(), None, "upstream_fill")
        {
            self.repository.persist_shred_tx(&record).await?;
        }

        Ok((transaction_value, HitSource::UpstreamFill))
    }

    async fn get_transaction_receipt(&self, tx_hash: &str) -> Result<(Value, HitSource)> {
        if let Some(record) = self.repository.redis().get_shred_tx(tx_hash).await? {
            if let Some(receipt) = parse_receipt_value(&record)? {
                return Ok((receipt, HitSource::Redis));
            }
        }

        if let Some(record) = self.repository.postgres().get_shred_tx(tx_hash).await? {
            if let Some(receipt) = parse_receipt_value(&record)? {
                self.repository.cache_shred_tx(&record).await?;
                return Ok((receipt, HitSource::Postgres));
            }
        }

        let receipt = self.upstream.get_transaction_receipt(tx_hash).await?;
        let Some(receipt_value) = receipt_to_value(receipt)? else {
            return Ok((Value::Null, HitSource::UpstreamFill));
        };

        let transaction = self.upstream.get_transaction_by_hash(tx_hash).await?;
        if let Some(transaction_value) = transaction {
            if let Some(record) = ShredTxRecord::from_rpc_values(
                transaction_value,
                Some(receipt_value.clone()),
                "upstream_fill",
            ) {
                self.repository.persist_shred_tx(&record).await?;
                self.repository.cache_shred_tx(&record).await?;
            }
        }

        Ok((receipt_value, HitSource::UpstreamFill))
    }
}

fn first_param_as_str(params: &Value) -> Option<String> {
    params
        .as_array()
        .and_then(|params| params.first())
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn success_response(id: Value, result: Value) -> Value {
    serde_json::to_value(JsonRpcSuccessResponse {
        jsonrpc: "2.0",
        id,
        result,
    })
    .unwrap_or_else(|_| json!({"jsonrpc":"2.0","id":Value::Null,"error":{"code":-32000,"message":"serialization error"}}))
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    serde_json::to_value(JsonRpcErrorResponse {
        jsonrpc: "2.0",
        id,
        error: JsonRpcErrorObject {
            code,
            message: message.to_string(),
        },
    })
    .unwrap_or_else(|_| json!({"jsonrpc":"2.0","id":Value::Null,"error":{"code":-32000,"message":"serialization error"}}))
}

fn hit_source_name(source: HitSource) -> &'static str {
    match source {
        HitSource::Redis => "redis_hit",
        HitSource::Postgres => "postgres_hit",
        HitSource::UpstreamFill => "upstream_fill",
    }
}

fn receipt_to_value(receipt: Option<crate::models::RpcReceipt>) -> Result<Option<Value>> {
    receipt
        .map(|receipt| {
            serde_json::from_str(&receipt.raw.to_string())
                .context("failed to serialize upstream receipt value")
        })
        .transpose()
}

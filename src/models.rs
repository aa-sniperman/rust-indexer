use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShredTxRecord {
    pub tx_hash: String,
    pub block_number: Option<i64>,
    pub block_timestamp: Option<i64>,
    pub block_hash: Option<String>,
    pub shred_idx: Option<i32>,
    pub tx_offset_in_shred: i32,
    pub starting_log_index: Option<i32>,
    pub signer: Option<String>,
    pub to_address: Option<String>,
    pub tx_type: Option<String>,
    pub receipt_status: Option<String>,
    pub transaction_json: String,
    pub receipt_json: String,
    pub state_changes_json: Option<String>,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackfillProgress {
    pub job_name: String,
    pub last_completed_block: i64,
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcSubscriptionMessage {
    pub method: String,
    pub params: SubscriptionParams,
}

#[derive(Debug, Deserialize)]
pub struct SubscriptionParams {
    pub subscription: String,
    pub result: ShredNotification,
}

#[derive(Debug, Deserialize)]
pub struct ShredNotification {
    #[serde(rename = "blockTimestamp")]
    pub block_timestamp: Option<i64>,
    #[serde(rename = "blockNumber", deserialize_with = "de_opt_i64_from_any")]
    pub block_number: Option<i64>,
    #[serde(rename = "shredIdx")]
    pub shred_idx: Option<i32>,
    #[serde(rename = "startingLogIndex", deserialize_with = "de_opt_i32_from_any")]
    pub starting_log_index: Option<i32>,
    pub transactions: Vec<ShredEnvelope>,
    #[serde(default, rename = "stateChanges")]
    pub state_changes: Value,
}

#[derive(Debug, Deserialize)]
pub struct ShredEnvelope {
    pub transaction: Value,
    pub receipt: Value,
}

#[derive(Debug, Deserialize)]
pub struct RpcBlock {
    pub number: Option<String>,
    pub timestamp: Option<String>,
    pub hash: Option<String>,
    #[serde(default)]
    pub transactions: Vec<RpcTransaction>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcTransaction {
    pub hash: String,
    #[serde(default, rename = "from")]
    pub from_address: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default, rename = "type")]
    pub tx_type: Option<String>,
    #[serde(default, rename = "transactionIndex")]
    pub transaction_index: Option<String>,
    #[serde(flatten)]
    pub raw: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcReceipt {
    #[serde(rename = "transactionHash")]
    pub transaction_hash: String,
    #[serde(default, rename = "status")]
    pub status: Option<String>,
    #[serde(default, rename = "blockHash")]
    pub block_hash: Option<String>,
    #[serde(default, rename = "transactionIndex")]
    pub transaction_index: Option<String>,
    #[serde(flatten)]
    pub raw: Value,
}

impl ShredTxRecord {
    pub fn from_shred_envelope(
        envelope: ShredEnvelope,
        block_number: Option<i64>,
        block_timestamp: Option<i64>,
        shred_idx: Option<i32>,
        starting_log_index: Option<i32>,
        tx_offset_in_shred: i32,
        state_changes: &Value,
        source: &str,
    ) -> Option<Self> {
        let tx_hash = envelope
            .transaction
            .get("hash")
            .and_then(Value::as_str)
            .map(str::to_owned)?;

        let block_hash = envelope
            .transaction
            .get("blockHash")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| {
                envelope
                    .receipt
                    .get("blockHash")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            });

        Some(Self {
            tx_hash,
            block_number,
            block_timestamp,
            block_hash,
            shred_idx,
            tx_offset_in_shred,
            starting_log_index,
            signer: envelope
                .transaction
                .get("signer")
                .or_else(|| envelope.transaction.get("from"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            to_address: envelope
                .transaction
                .get("to")
                .and_then(Value::as_str)
                .map(str::to_owned),
            tx_type: envelope
                .transaction
                .get("type")
                .and_then(value_to_lossy_string),
            receipt_status: envelope
                .receipt
                .get("status")
                .and_then(value_to_lossy_string),
            transaction_json: envelope.transaction.to_string(),
            receipt_json: envelope.receipt.to_string(),
            state_changes_json: if state_changes.is_null() {
                None
            } else {
                Some(state_changes.to_string())
            },
            source: source.to_owned(),
        })
    }

    pub fn from_backfill(
        block: &RpcBlock,
        transaction: RpcTransaction,
        receipt: RpcReceipt,
        source: &str,
    ) -> Self {
        let block_number = block.number.as_deref().and_then(parse_i64_from_str);
        let block_timestamp = block.timestamp.as_deref().and_then(parse_i64_from_str);
        let tx_offset_in_shred = transaction
            .transaction_index
            .as_deref()
            .and_then(parse_optional_i32_str)
            .unwrap_or_default();

        Self {
            tx_hash: transaction.hash.clone(),
            block_number,
            block_timestamp,
            block_hash: receipt.block_hash.clone().or_else(|| block.hash.clone()),
            shred_idx: None,
            tx_offset_in_shred,
            starting_log_index: None,
            signer: transaction.from_address.clone(),
            to_address: transaction.to.clone(),
            tx_type: transaction.tx_type.clone(),
            receipt_status: receipt.status.clone(),
            transaction_json: transaction.raw.to_string(),
            receipt_json: receipt.raw.to_string(),
            state_changes_json: None,
            source: source.to_owned(),
        }
    }
}

fn parse_optional_i32_value(value: &Value) -> Option<i32> {
    parse_optional_i64_value(value).and_then(|v| i32::try_from(v).ok())
}

fn parse_optional_i64_value(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => parse_i64_from_str(text),
        _ => None,
    }
}

fn parse_optional_i32_str(input: &str) -> Option<i32> {
    parse_i64_from_str(input).and_then(|v| i32::try_from(v).ok())
}

fn value_to_lossy_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        _ => Some(value.to_string()),
    }
}

fn parse_i64_from_str(input: &str) -> Option<i64> {
    if let Some(hex) = input.strip_prefix("0x") {
        i64::from_str_radix(hex, 16).ok()
    } else {
        input.parse::<i64>().ok()
    }
}

fn de_opt_i64_from_any<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(value.as_ref().and_then(parse_optional_i64_value))
}

fn de_opt_i32_from_any<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(value.as_ref().and_then(parse_optional_i32_value))
}

#[cfg(test)]
mod tests {
    use super::{RpcBlock, RpcReceipt, RpcTransaction, ShredEnvelope, ShredTxRecord};

    #[test]
    fn builds_shred_native_record_from_payload() {
        let envelope = ShredEnvelope {
            transaction: serde_json::json!({
                "hash": "0xabc",
                "signer": "0xfrom",
                "to": "0xto",
                "type": "0x2"
            }),
            receipt: serde_json::json!({
                "status": "0x1",
                "cumulativeGasUsed": "0x5208",
                "logs": [],
                "type": "0x2"
            }),
        };

        let indexed = ShredTxRecord::from_shred_envelope(
            envelope,
            Some(123),
            Some(456),
            Some(7),
            Some(99),
            0,
            &serde_json::json!({"0xabc":{"nonce":1}}),
            "realtime",
        )
        .expect("payload should yield record");

        assert_eq!(indexed.tx_hash, "0xabc");
        assert_eq!(indexed.signer.as_deref(), Some("0xfrom"));
        assert_eq!(indexed.tx_offset_in_shred, 0);
        assert_eq!(indexed.receipt_status.as_deref(), Some("0x1"));
    }

    #[test]
    fn builds_backfill_record_from_rpc_payload() {
        let block = RpcBlock {
            number: Some("0x2a".to_string()),
            timestamp: Some("0x64".to_string()),
            hash: Some("0xblockhash".to_string()),
            transactions: vec![],
        };
        let transaction = RpcTransaction {
            hash: "0xtx".to_string(),
            from_address: Some("0xfrom".to_string()),
            to: Some("0xto".to_string()),
            tx_type: Some("0x2".to_string()),
            transaction_index: Some("0x3".to_string()),
            raw: serde_json::json!({"hash":"0xtx","from":"0xfrom","to":"0xto","type":"0x2","transactionIndex":"0x3"}),
        };
        let receipt = RpcReceipt {
            transaction_hash: "0xtx".to_string(),
            status: Some("0x1".to_string()),
            block_hash: Some("0xblockhash".to_string()),
            transaction_index: Some("0x3".to_string()),
            raw: serde_json::json!({"transactionHash":"0xtx","status":"0x1","blockHash":"0xblockhash","transactionIndex":"0x3"}),
        };

        let record = ShredTxRecord::from_backfill(&block, transaction, receipt, "backfill");
        assert_eq!(record.block_number, Some(42));
        assert_eq!(record.tx_offset_in_shred, 3);
        assert_eq!(record.source, "backfill");
    }
}

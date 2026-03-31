CREATE TABLE IF NOT EXISTS shred_transactions (
    tx_hash TEXT PRIMARY KEY,
    block_number BIGINT,
    block_timestamp BIGINT,
    block_hash TEXT,
    shred_idx INTEGER,
    tx_offset_in_shred INTEGER,
    starting_log_index INTEGER,
    signer TEXT,
    to_address TEXT,
    tx_type TEXT,
    receipt_status TEXT,
    transaction_json TEXT NOT NULL,
    receipt_json TEXT NOT NULL,
    state_changes_json TEXT,
    source TEXT NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_shred_transactions_block_number
    ON shred_transactions (block_number);

CREATE INDEX IF NOT EXISTS idx_shred_transactions_shred_position
    ON shred_transactions (block_number, shred_idx, tx_offset_in_shred);

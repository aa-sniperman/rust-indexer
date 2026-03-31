CREATE TABLE IF NOT EXISTS transactions (
    tx_hash TEXT PRIMARY KEY,
    block_number BIGINT,
    block_hash TEXT,
    tx_index INTEGER,
    from_address TEXT,
    to_address TEXT,
    raw_json TEXT NOT NULL,
    source TEXT NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_transactions_block_number ON transactions (block_number);

CREATE TABLE IF NOT EXISTS receipts (
    tx_hash TEXT PRIMARY KEY,
    block_number BIGINT,
    block_hash TEXT,
    tx_index INTEGER,
    status TEXT,
    contract_address TEXT,
    gas_used TEXT,
    cumulative_gas_used TEXT,
    logs_json TEXT NOT NULL,
    raw_json TEXT NOT NULL,
    source TEXT NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_receipts_block_number ON receipts (block_number);

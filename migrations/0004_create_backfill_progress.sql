CREATE TABLE IF NOT EXISTS backfill_progress (
    job_name TEXT PRIMARY KEY,
    last_completed_block BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ -f "${ROOT_DIR}/.env" ]]; then
  set -a
  source "${ROOT_DIR}/.env"
  set +a
fi

if [[ $# -ne 2 ]]; then
  echo "Usage: $0 <from_block> <to_block>"
  exit 1
fi

FROM_BLOCK="$1"
TO_BLOCK="$2"
JOB_NAME="backfill"

if [[ -z "${POSTGRES_URL:-}" ]]; then
  echo "POSTGRES_URL is missing. Set it in .env first."
  exit 1
fi

psql "${POSTGRES_URL}" <<SQL
\echo 'Backfill row counts'
SELECT
  COUNT(*) AS total_rows,
  COUNT(*) FILTER (WHERE source = 'backfill') AS backfill_rows,
  MIN(block_number) AS min_block,
  MAX(block_number) AS max_block
FROM shred_transactions
WHERE block_number BETWEEN ${FROM_BLOCK} AND ${TO_BLOCK};

\echo ''
\echo 'Backfill progress'
SELECT
  job_name,
  last_completed_block,
  updated_at
FROM backfill_progress
WHERE job_name = '${JOB_NAME}';

\echo ''
\echo 'Sample rows'
SELECT
  tx_hash,
  block_number,
  signer,
  to_address,
  receipt_status,
  source
FROM shred_transactions
WHERE block_number BETWEEN ${FROM_BLOCK} AND ${TO_BLOCK}
ORDER BY block_number, tx_offset_in_shred
LIMIT 10;
SQL

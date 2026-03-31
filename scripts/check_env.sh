#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ ! -f "${ROOT_DIR}/.env" ]]; then
  echo ".env is missing. Copy .env.example to .env first."
  exit 1
fi

required_vars=(
  RISE_HTTP_RPC_URL
  RISE_WS_RPC_URL
  POSTGRES_URL
  REDIS_URL
  SERVER_BIND_ADDR
  BACKFILL_START_BLOCK
  BACKFILL_BATCH_SIZE
  REDIS_TTL_SECS
  LOG_LEVEL
)

set -a
source "${ROOT_DIR}/.env"
set +a

for var_name in "${required_vars[@]}"; do
  if [[ -z "${!var_name:-}" ]]; then
    echo "Missing required env var: ${var_name}"
    exit 1
  fi
done

echo "Environment looks valid."

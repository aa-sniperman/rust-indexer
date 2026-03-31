#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ -f "${ROOT_DIR}/.env" ]]; then
  set -a
  source "${ROOT_DIR}/.env"
  set +a
fi

if [[ $# -gt 1 ]]; then
  echo "Usage: $0 [start_block]"
  exit 1
fi

START_BLOCK="${1:-${BACKFILL_START_BLOCK:-}}"

if [[ -z "${POSTGRES_URL:-}" || -z "${REDIS_URL:-}" || -z "${RISE_HTTP_RPC_URL:-}" || -z "${RISE_WS_RPC_URL:-}" ]]; then
  echo "Required env vars are missing. Check .env first."
  exit 1
fi

if [[ -z "${START_BLOCK:-}" ]]; then
  echo "BACKFILL_START_BLOCK is missing. Set it in .env or pass [start_block]."
  exit 1
fi

BACKFILL_START_BLOCK="${START_BLOCK}" \
cargo run --manifest-path "${ROOT_DIR}/Cargo.toml" --bin backfill

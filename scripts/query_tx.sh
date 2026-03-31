#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ -f "${ROOT_DIR}/.env" ]]; then
  set -a
  source "${ROOT_DIR}/.env"
  set +a
fi

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <tx_hash>"
  exit 1
fi

if [[ -z "${SERVER_BIND_ADDR:-}" ]]; then
  echo "SERVER_BIND_ADDR is missing. Set it in .env first."
  exit 1
fi

curl -sS \
  -X POST "http://${SERVER_BIND_ADDR}" \
  -H 'content-type: application/json' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"eth_getTransactionByHash\",\"params\":[\"$1\"]}"
echo

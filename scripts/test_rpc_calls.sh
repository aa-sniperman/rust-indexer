#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ -f "${ROOT_DIR}/.env" ]]; then
  set -a
  source "${ROOT_DIR}/.env"
  set +a
fi

SERVER_ADDR="${SERVER_BIND_ADDR:-127.0.0.1:3000}"
BASE_URL="http://${SERVER_ADDR}"
TX_HASH="${1:-0x0000000000000000000000000000000000000000000000000000000000000000}"

echo "== Health Check =="
curl -sS "${BASE_URL}/healthz"
echo
echo

echo "== eth_getTransactionByHash =="
curl -sS \
  -X POST "${BASE_URL}" \
  -H 'content-type: application/json' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"eth_getTransactionByHash\",\"params\":[\"${TX_HASH}\"]}"
echo
echo

echo "== eth_getTransactionReceipt =="
curl -sS \
  -X POST "${BASE_URL}" \
  -H 'content-type: application/json' \
  -d "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"eth_getTransactionReceipt\",\"params\":[\"${TX_HASH}\"]}"
echo
echo

echo "== eth_blockNumber passthrough =="
curl -sS \
  -X POST "${BASE_URL}" \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":3,"method":"eth_blockNumber","params":[]}'
echo
echo

echo "== eth_chainId passthrough =="
curl -sS \
  -X POST "${BASE_URL}" \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":4,"method":"eth_chainId","params":[]}'
echo
echo

echo "== web3_clientVersion passthrough =="
curl -sS \
  -X POST "${BASE_URL}" \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":5,"method":"web3_clientVersion","params":[]}'
echo

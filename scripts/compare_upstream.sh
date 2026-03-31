#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ -f "${ROOT_DIR}/.env" ]]; then
  set -a
  source "${ROOT_DIR}/.env"
  set +a
fi

if [[ $# -ne 2 ]]; then
  echo "Usage: $0 <method> <params_json>"
  exit 1
fi

METHOD="$1"
PARAMS_JSON="$2"

if [[ -z "${SERVER_BIND_ADDR:-}" || -z "${RISE_HTTP_RPC_URL:-}" ]]; then
  echo "SERVER_BIND_ADDR or RISE_HTTP_RPC_URL is missing. Set them in .env first."
  exit 1
fi

REQUEST_BODY="$(printf '{"jsonrpc":"2.0","id":1,"method":"%s","params":%s}' "${METHOD}" "${PARAMS_JSON}")"

echo "Local:"
curl -sS \
  -X POST "http://${SERVER_BIND_ADDR}" \
  -H 'content-type: application/json' \
  -d "${REQUEST_BODY}"
echo
echo
echo "Upstream:"
curl -sS \
  -X POST "${RISE_HTTP_RPC_URL}" \
  -H 'content-type: application/json' \
  -d "${REQUEST_BODY}"
echo

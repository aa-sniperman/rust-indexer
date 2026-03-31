#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ -f "${ROOT_DIR}/.env" ]]; then
  set -a
  source "${ROOT_DIR}/.env"
  set +a
fi

if [[ -z "${REDIS_URL:-}" ]]; then
  echo "REDIS_URL is missing. Set it in .env first."
  exit 1
fi

redis-cli -u "${REDIS_URL}" FLUSHALL

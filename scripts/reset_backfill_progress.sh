#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ -f "${ROOT_DIR}/.env" ]]; then
  set -a
  source "${ROOT_DIR}/.env"
  set +a
fi

JOB_NAME="${1:-backfill}"

if [[ -z "${POSTGRES_URL:-}" ]]; then
  echo "POSTGRES_URL is missing. Set it in .env first."
  exit 1
fi

psql "${POSTGRES_URL}" -c "DELETE FROM backfill_progress WHERE job_name = '${JOB_NAME}';"

echo "Reset backfill progress for job '${JOB_NAME}'."

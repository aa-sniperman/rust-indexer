#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

docker compose -f "${ROOT_DIR}/docker-compose.yml" down -v
docker compose -f "${ROOT_DIR}/docker-compose.yml" up -d
cargo run --manifest-path "${ROOT_DIR}/Cargo.toml" --bin migrate

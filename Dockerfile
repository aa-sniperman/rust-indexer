FROM rust:1.88-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN cargo build --release --bin indexer --bin backfill --bin rpc_server --bin migrate

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/indexer /usr/local/bin/indexer
COPY --from=builder /app/target/release/backfill /usr/local/bin/backfill
COPY --from=builder /app/target/release/rpc_server /usr/local/bin/rpc_server
COPY --from=builder /app/target/release/migrate /usr/local/bin/migrate
COPY migrations ./migrations


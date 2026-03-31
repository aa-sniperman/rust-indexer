# RISE Indexer PoC

## 1. What This PoC Achieves

| Area | Implemented In This PoC | Purpose |
| --- | --- | --- |
| Real-time ingest | Subscribe `shreds` over WS and index new transactions continuously | Keep fresh transactions queryable with low latency |
| Best-effort backfill | Resume from a Postgres checkpoint, scan blocks, skip already indexed txs, and keep polling for newly finalized historical gaps | Fill historical gaps without blocking realtime indexing |
| Heavy-block backfill policy | If a block has too many missing txs, store tx-only and defer receipt fetch to user-driven reads | Keep backfill practical when receipt fan-out would be too slow |
| Durable local index | Store shred-native records and backfill progress in `Postgres` | Maintain a local durable source for local-first serving |
| Hot cache | Cache hot lookups in `Redis` with TTL and `allkeys-lfu` | Prioritize low-latency reads for recent and frequently requested txs |
| Local-first JSON-RPC | Serve `eth_getTransactionByHash` and `eth_getTransactionReceipt` from Redis/Postgres first, then upstream-fill on miss | Reduce expensive upstream RPC dependence |
| Fallback RPC proxy | Forward all other JSON-RPC methods to the upstream RISE testnet node | Keep the server useful without implementing a full node surface |
| Service split | Run realtime ingestion, backfill, and JSON-RPC serving as separate services in one compose stack | Keep responsibilities isolated while staying simple for a PoC |
| Explicit scope control | Reorg handling, benchmarking, monitoring, and comprehensive testing remain out of scope | Keep the deliverable focused and reviewable in a short take-home window |

`Postgres` is used here as a pragmatic PoC choice. Long term, a distributed store such as `Scylla` or `Cassandra` could be a better fit, since this workload is append-heavy and primarily queried by `tx_hash`, which maps naturally to a partition key for fast distributed lookups.

## 2. Run And Demo

Prepare the environment file first:

```bash
cp .env.example .env
```

By default, `BACKFILL_START_BLOCK=0` means: if there is no backfill checkpoint yet, the backfiller starts from `latest_block - 1` instead of from genesis.

Start the full stack:

```bash
docker compose up --build
```

That starts:
- `realtime-ingestor`
- `backfiller`
- `json-rpc-server`
- `postgres`
- `redis`

The binaries already run migrations on startup, so there is no extra bootstrap step.

For the best demo, pick the newest transaction hash you can find from a RISE Testnet explorer. A very recent tx is the best signal because it shows how quickly the realtime ingestor is pulling new data into the local stack.

Call the local JSON-RPC server with:

```bash
./scripts/test_rpc_calls.sh <tx_hash>
```

Recommended reviewer flow:
1. Find the newest tx hash available on a RISE Testnet explorer.
2. Run `./scripts/test_rpc_calls.sh <tx_hash>`. => see how the server returns
3. Check the `json-rpc-server` logs.

What to look for in the logs:
- `redis_hit`: best case, the tx is already hot in cache.
- `postgres_hit`: still local, but served from the durable index and then re-cached.
- `upstream_fill`: the tx was not local yet, so the server fetched upstream and indexed it on demand.

If you want to observe all paths more explicitly:
1. Use a very recent tx first. This is the best way to see the system indexing fresh chain activity quickly.
2. Query an older tx that is probably not in local cache yet to trigger `upstream_fill`.
3. Query the same tx again to see `redis_hit`.
4. Flush Redis with `./scripts/flush_redis.sh`, then query again to see `postgres_hit`.

If I were reviewing this PoC, I would strongly prefer testing with the newest txs available on testnet, because that best demonstrates the realtime ingestion path rather than only the fallback path.

## 3. Next Improvements

| Priority | Improvement | Why It Matters |
| --- | --- | --- |
| 1 | Stronger backpressure and flow control between the WS listener and the cache/durable workers | This is the highest-value next step because sustained realtime throughput will stress queues before it stresses business logic |
| 2 | Stronger reconnect policy for the realtime WS client | A production version should handle disconnects, stale sockets, and re-subscription more deliberately |
| 3 | Smarter historical ingestion throughput | The current backfill is intentionally pragmatic; a larger-scale version would need more aggressive scheduling and receipt hydration policies |
| 4 | A more scalable durable store if throughput requirements grow substantially | `Postgres` is a good PoC choice, but an append-heavy, distributed workload may later justify something like `Scylla/Cassandra` |

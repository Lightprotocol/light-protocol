# Forester

## Description

Forester is a service that processes queued Merkle tree updates for Light Protocol's ZK Compression system on Solana. It reads pending nullifications and address insertions from queue accounts and submits batched transactions with ZK proofs to update the on-chain Merkle trees.

## Quick Start

1. Copy the example environment file:
```bash
cp .env.example .env
```

2. Configure your environment variables (see Configuration section)

3. Start the forester:
```bash
cargo run -- start
# or with environment file
source .env && cargo run -- start
```

## Commands

```bash
forester <COMMAND>
```

| Command | Description |
|---------|-------------|
| `start` | Start the Forester service |
| `status` | Check queue and protocol status |
| `health` | Run health checks (balance, registration) |
| `dashboard` | Run a standalone API server (no processing) |

## Configuration

All configuration is provided via CLI arguments or environment variables. There is **no prefix** on env vars (e.g. `RPC_URL`, not `FORESTER_RPC_URL`).

### Required

| Option | Env Var | Description |
|--------|---------|-------------|
| `--rpc-url` | `RPC_URL` | Solana RPC endpoint |
| `--indexer-url` | `INDEXER_URL` | Photon indexer URL (supports `?api-key=KEY`) |
| `--payer` | `PAYER` | Keypair for signing (JSON byte array) |
| `--derivation` | `DERIVATION_PUBKEY` | Derivation pubkey (JSON byte array, 32 bytes) |

### Optional Services

| Option | Env Var | Description |
|--------|---------|-------------|
| `--ws-rpc-url` | `WS_RPC_URL` | WebSocket RPC (required for `--enable-compressible`) |
| `--prover-url` | `PROVER_URL` | ZK prover service URL |
| `--prover-append-url` | `PROVER_APPEND_URL` | Prover URL for append ops (falls back to `--prover-url`) |
| `--prover-update-url` | `PROVER_UPDATE_URL` | Prover URL for update ops (falls back to `--prover-url`) |
| `--prover-address-append-url` | `PROVER_ADDRESS_APPEND_URL` | Prover URL for address-append ops (falls back to `--prover-url`) |
| `--prover-api-key` | `PROVER_API_KEY` | API key for the prover service |
| `--photon-grpc-url` | `PHOTON_GRPC_URL` | Photon gRPC endpoint |

### Resilience & Fallback

When a fallback RPC URL is configured, the pool automatically switches to it if the primary becomes unhealthy, and switches back when the primary recovers.

| Option | Env Var | Default | Description |
|--------|---------|---------|-------------|
| `--fallback-rpc-url` | `FALLBACK_RPC_URL` | | Fallback Solana RPC endpoint |
| `--fallback-indexer-url` | `FALLBACK_INDEXER_URL` | | Fallback Photon indexer URL |
| `--rpc-pool-failure-threshold` | `RPC_POOL_FAILURE_THRESHOLD` | 3 | Consecutive health check failures before switching to fallback |
| `--rpc-pool-primary-probe-interval-secs` | `RPC_POOL_PRIMARY_PROBE_INTERVAL_SECS` | 30 | Seconds between probes to check if primary has recovered |

**How it works:**
1. The connection pool validates each connection via Solana `getHealth` before use
2. After `failure_threshold` consecutive failures, all new connections route to the fallback URL
3. Existing primary connections are eagerly dropped so they get replaced with fallback connections
4. A background probe checks the primary every `primary_probe_interval_secs` seconds and auto-recovers

### RPC Pool

| Option | Env Var | Default | Description |
|--------|---------|---------|-------------|
| `--rpc-pool-size` | `RPC_POOL_SIZE` | 100 | Number of pooled RPC connections |
| `--rpc-pool-connection-timeout-secs` | `RPC_POOL_CONNECTION_TIMEOUT_SECS` | 15 | Connection timeout |
| `--rpc-pool-idle-timeout-secs` | `RPC_POOL_IDLE_TIMEOUT_SECS` | 300 | Idle connection timeout |
| `--rpc-pool-max-retries` | `RPC_POOL_MAX_RETRIES` | 100 | Max retries to get a connection from the pool |
| `--rpc-pool-initial-retry-delay-ms` | `RPC_POOL_INITIAL_RETRY_DELAY_MS` | 1000 | Initial backoff delay |
| `--rpc-pool-max-retry-delay-ms` | `RPC_POOL_MAX_RETRY_DELAY_MS` | 16000 | Max backoff delay |

### Processing

| Option | Env Var | Default | Description |
|--------|---------|---------|-------------|
| `--processor-mode` | `PROCESSOR_MODE` | `all` | `v1`, `v2`, or `all` |
| `--queue-polling-mode` | `QUEUE_POLLING_MODE` | `indexer` | `indexer` or `onchain` |
| `--tree-id` | `TREE_IDS` | | Process only these tree pubkeys (comma-separated) |
| `--group-authority` | `GROUP_AUTHORITY` | | Only process trees owned by this authority |
| `--max-concurrent-sends` | `MAX_CONCURRENT_SENDS` | 50 | Concurrent transaction sends per batch |
| `--transaction-max-concurrent-batches` | `TRANSACTION_MAX_CONCURRENT_BATCHES` | 20 | Concurrent transaction batches |
| `--max-batches-per-tree` | `MAX_BATCHES_PER_TREE` | 4 | Max ZKP batches per tree per iteration |
| `--legacy-ixs-per-tx` | `LEGACY_IXS_PER_TX` | 1 | Instructions per V1 transaction |
| `--cu-limit` | `CU_LIMIT` | 1000000 | Compute unit limit per transaction |
| `--enable-priority-fees` | `ENABLE_PRIORITY_FEES` | false | Enable dynamic priority fees |
| `--lookup-table-address` | `LOOKUP_TABLE_ADDRESS` | | Address lookup table for versioned transactions |
| `--helius-rpc` | `HELIUS_RPC` | false | Use Helius `getProgramAccountsV2` |

### Compressible Accounts

| Option | Env Var | Default | Description |
|--------|---------|---------|-------------|
| `--enable-compressible` | `ENABLE_COMPRESSIBLE` | false | Enable compressible account tracking (requires `--ws-rpc-url`) |
| `--light-pda-program` | `LIGHT_PDA_PROGRAMS` | | PDA programs to track (`program_id:discriminator_base58`, comma-separated) |

### Caching & Confirmation

| Option | Env Var | Default | Description |
|--------|---------|---------|-------------|
| `--tx-cache-ttl-seconds` | `TX_CACHE_TTL_SECONDS` | 180 | Transaction deduplication cache TTL |
| `--ops-cache-ttl-seconds` | `OPS_CACHE_TTL_SECONDS` | 180 | Operations cache TTL |
| `--confirmation-max-attempts` | `CONFIRMATION_MAX_ATTEMPTS` | 60 | Max tx confirmation polling attempts |
| `--confirmation-poll-interval-ms` | `CONFIRMATION_POLL_INTERVAL_MS` | 500 | Confirmation polling interval |

### Monitoring

| Option | Env Var | Description |
|--------|---------|-------------|
| `--push-gateway-url` | `PUSH_GATEWAY_URL` | Prometheus Pushgateway URL (enables metrics) |
| `--pagerduty-routing-key` | `PAGERDUTY_ROUTING_KEY` | PagerDuty integration key |
| `--prometheus-url` | `PROMETHEUS_URL` | Prometheus server URL for dashboard queries |
| `--api-server-port` | `API_SERVER_PORT` | HTTP API server port (default: 8080) |
| `--api-server-public-bind` | `API_SERVER_PUBLIC_BIND` | Bind to 0.0.0.0 instead of 127.0.0.1 |

### Example

```bash
cargo run -- start \
  --rpc-url "$RPC_URL" \
  --indexer-url "$INDEXER_URL" \
  --prover-url "$PROVER_URL" \
  --payer "$PAYER" \
  --derivation "$DERIVATION_PUBKEY" \
  --fallback-rpc-url "$FALLBACK_RPC_URL" \
  --fallback-indexer-url "$FALLBACK_INDEXER_URL" \
  --rpc-pool-size 100 \
  --processor-mode v2 \
  --max-concurrent-sends 200 \
  --cu-limit 400000 \
  --enable-priority-fees true
```

## Status & Health

```bash
# Check queue and protocol status
forester status --rpc-url <RPC_URL> [--full] [--protocol-config] [--queue]

# Health checks
forester health --rpc-url <RPC_URL> --payer <PAYER> --derivation <DERIVATION> \
  [--check-balance] [--check-registration] [--min-balance 0.01]
```

## Dashboard

Run a standalone API server without forester processing:

```bash
forester dashboard --rpc-url <RPC_URL> \
  [--port 8080] \
  [--prometheus-url http://prometheus:9090] \
  [--forester-api-url http://forester-a:8080,http://forester-b:8080]
```

## Environment File

See `.env.example` for a complete example configuration.

## Testing

See the [main CLAUDE.md](../CLAUDE.md) for test commands. Key forester-specific tests:

```bash
# E2E test (requires local validator)
TEST_MODE=local cargo test --package forester e2e_test -- --nocapture

# Metrics contract test
cargo test -p forester --test metrics_contract_test -- --nocapture
```

### Test Environment Variables

| Variable | Description |
|----------|-------------|
| `TEST_MODE` | `local` or `devnet` (default: `devnet`) |
| `TEST_V1_STATE` | Enable V1 state tree tests (default: `true`) |
| `TEST_V2_STATE` | Enable V2 state tree tests (default: `true`) |
| `TEST_V1_ADDRESS` | Enable V1 address tree tests (default: `true`) |
| `TEST_V2_ADDRESS` | Enable V2 address tree tests (default: `true`) |
| `FORESTER_KEYPAIR` | Test keypair (base58 or byte array format) |

# Forester

## Description

Forester is a service for nullifying the state and address merkle trees.
It subscribes to the nullifier queue and nullifies merkle tree leaves.


## Quick Start

1. Copy the example environment file:
```bash
cp .env.example .env
```

2. Configure your environment variables (see Configuration section)

3. Start the forester:
```bash
cargo run start
# or with environment file
source .env && cargo run start
```

## Commands

```bash
forester <COMMAND>
```

Available commands:
- `start` - Start the Forester service
- `status` - Check the status of various components
- `health` - Perform health checks on the system
- `help` - Print help information

## Configuration

All configuration can be provided via command-line arguments or environment variables. Environment variables take the format `FORESTER_<OPTION_NAME>`.

### Required Configuration

| Option | Environment Variable | Description |
|--------|---------------------|-------------|
| `--rpc-url` | `FORESTER_RPC_URL` | Solana RPC endpoint URL |
| `--ws-rpc-url` | `FORESTER_WS_RPC_URL` | WebSocket RPC endpoint URL |
| `--indexer-url` | `FORESTER_INDEXER_URL` | Photon indexer URL |
| `--prover-url` | `FORESTER_PROVER_URL` | Light Protocol prover service URL |
| `--payer` | `FORESTER_PAYER` | Keypair for transaction signing (JSON array format) |
| `--derivation` | `FORESTER_DERIVATION_PUBKEY` | Derivation public key (JSON array format) |

### Performance Configuration

#### RPC Pool Settings
Optimize connection pooling for better throughput:

| Option | Default | Description |
|--------|---------|-------------|
| `--rpc-pool-size` | 10 | Number of RPC connections to maintain |
| `--rpc-pool-connection-timeout-secs` | 15 | Connection timeout in seconds |
| `--rpc-pool-idle-timeout-secs` | 300 | Idle connection timeout |

#### Transaction V1 Processing
Control transaction batching and concurrency:

| Option | Default | Description |
|--------|---------|-------------|
| `--max-concurrent-sends` | 50 | Maximum concurrent transaction sends |
| `--legacy-ixs-per-tx` | 1 | Instructions per transaction (max 1 for address nullification) |
| `--transaction-max-concurrent-batches` | 20 | Maximum concurrent transaction batches |
| `--cu-limit` | 1000000 | Compute unit limit per transaction |
| `--enable-priority-fees` | false | Enable dynamic priority fee calculation |

#### Example

```bash
cargo run start \
  --rpc-url "$RPC_URL" \
  --ws-rpc-url "$WS_RPC_URL" \
  --indexer-url "$INDEXER_URL" \
  --prover-url "$PROVER_URL" \
  --payer "$FORESTER_KEYPAIR" \
  --derivation "$FORESTER_DERIVATION" \
  --rpc-pool-size 100 \
  --max-concurrent-sends 500 \
  --cu-limit 400000 \
  --enable-priority-fees true
```


### Prover V2 Endpoints

```bash
--prover-append-url "http://prover/append"
--prover-update-url "http://prover/update"
--prover-address-append-url "http://prover/address-append"
--prover-api-key "your-api-key"
```

#### Cache Settings
Control caching behavior:

```bash
--tx-cache-ttl-seconds 180  # Transaction deduplication cache
--ops-cache-ttl-seconds 180 # Operations cache
```

### Environment File

See `.env.example` for a complete list of configuration options with example values.

## Checking Status

To check the status of Forester:

```bash
forester status [OPTIONS] --rpc-url <RPC_URL>
```

### Status Options:

- `--full` - Run comprehensive status checks including compressed token program tests
- `--protocol-config` - Check protocol configuration
- `--queue` - Check queue status
- `--push-gateway-url` - Monitoring push gateway URL [env: FORESTER_PUSH_GATEWAY_URL]
- `--pagerduty-routing-key` - PagerDuty integration key [env: FORESTER_PAGERDUTY_ROUTING_KEY]

## Environment Variables

All configuration options can be set using environment variables with the `FORESTER_` prefix. For example:

```bash
export FORESTER_RPC_URL="your-rpc-url-here"
```

### Test Environment Variables

The following environment variables are used for running the e2e_v2 tests:

#### Test Mode

- `TEST_MODE` - Specifies whether to run tests on local validator or devnet (values: `local` or `devnet`, default: `devnet`)

#### Test Feature Flags

Control which test scenarios to run (all default to `true`):

- `TEST_V1_STATE` - Enable/disable V1 state tree testing (`true`/`false`)
- `TEST_V2_STATE` - Enable/disable V2 state tree testing (`true`/`false`)
- `TEST_V1_ADDRESS` - Enable/disable V1 address tree testing (`true`/`false`)
- `TEST_V2_ADDRESS` - Enable/disable V2 address tree testing (`true`/`false`)

#### Required for Devnet mode:

- `PHOTON_RPC_URL` - Photon RPC endpoint URL
- `PHOTON_WSS_RPC_URL` - Photon WebSocket RPC endpoint URL
- `PHOTON_INDEXER_URL` - Photon indexer endpoint URL
- `PHOTON_PROVER_URL` - Photon prover endpoint URL
- `PHOTON_API_KEY` - Photon API key for authentication

#### Required for both modes:

- `FORESTER_KEYPAIR` - Keypair for testing (supports both base58 format and byte array format like `[1,2,3,...]`)

#### Example configurations:

**Local validator mode with all tests:**
```bash
export TEST_MODE="local"
export FORESTER_KEYPAIR="your-base58-encoded-keypair"
# OR using byte array format:
# export FORESTER_KEYPAIR="[1,2,3,...]"
```

**Local validator mode with only V1 tests:**
```bash
export TEST_MODE="local"
export TEST_V1_STATE="true"
export TEST_V2_STATE="false"
export TEST_V1_ADDRESS="true"
export TEST_V2_ADDRESS="false"
export FORESTER_KEYPAIR="your-base58-encoded-keypair"
```

**Devnet mode with only V2 tests:**
```bash
export TEST_MODE="devnet"
export TEST_V1_STATE="false"
export TEST_V2_STATE="true"
export TEST_V1_ADDRESS="false"
export TEST_V2_ADDRESS="true"
export PHOTON_RPC_URL="https://devnet.helius-rpc.com/?api-key=your-key"
export PHOTON_WSS_RPC_URL="wss://devnet.helius-rpc.com/?api-key=your-key"
export PHOTON_INDEXER_URL="https://devnet.helius-rpc.com"
export PHOTON_PROVER_URL="https://devnet.helius-rpc.com"
export PHOTON_API_KEY="your-api-key"
export FORESTER_KEYPAIR="your-base58-encoded-keypair"
```

When running in local mode, the test will:
- Spawn a local validator
- Start a local prover service
- Use predefined local URLs (localhost:8899 for RPC, localhost:8784 for indexer, etc.)

The test will automatically:
- Skip minting tokens for disabled test types
- Skip executing transactions for disabled test types
- Skip root verification for disabled test types

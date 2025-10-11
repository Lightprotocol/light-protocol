# Forester

## Description

Forester is a service for nullifying the state and address merkle trees.
It subscribes to the nullifier queue and nullifies merkle tree leaves.

## Commands

The Forester service can be controlled using the following syntax:

```bash
forester <COMMAND>
```

Available commands:

- `start` - Start the Forester service
- `status` - Check the status of various components
- `help` - Print help information for commands

## Starting the Service

To start Forester, use:

```bash
forester start [OPTIONS]
```

### Configuration Options

The start command supports the following configuration options, which can be set via command-line arguments or environment variables:

#### Required Options:

- `--rpc-url` - RPC URL [env: FORESTER_RPC_URL]
- `--ws-rpc-url` - WebSocket RPC URL [env: FORESTER_WS_RPC_URL]
- `--indexer-url` - Indexer URL [env: FORESTER_INDEXER_URL]
- `--prover-url` - Prover URL [env: FORESTER_PROVER_URL]
- `--payer` - Payer configuration [env: FORESTER_PAYER]
- `--derivation` - Derivation public key [env: FORESTER_DERIVATION_PUBKEY]

#### Optional Settings:

- `--push-gateway-url` - Monitoring gateway URL [env: FORESTER_PUSH_GATEWAY_URL]
- `--pagerduty-routing-key` - PagerDuty integration key [env: FORESTER_PAGERDUTY_ROUTING_KEY]
- `--photon-api-key` - Photon API key [env: FORESTER_PHOTON_API_KEY]

#### Performance Tuning:

- `--indexer-batch-size` - Size of indexer batches [default: 50]
- `--indexer-max-concurrent-batches` - Maximum concurrent indexer batches [default: 10]
- `--transaction-batch-size` - Size of transaction batches [default: 1]
- `--transaction-max-concurrent-batches` - Maximum concurrent transaction batches [default: 20]
- `--cu-limit` - Compute unit limit [default: 1000000]
- `--rpc-pool-size` - RPC connection pool size [default: 20]

#### Timing Configuration:

- `--slot-update-interval-seconds` - Interval for slot updates [default: 10]
- `--tree-discovery-interval-seconds` - Interval for tree discovery [default: 5]
- `--retry-delay` - Delay between retries in milliseconds [default: 1000]
- `--retry-timeout` - Timeout for retries in milliseconds [default: 30000]

#### Queue Configuration:

- `--state-queue-start-index` - Starting index for state queue [default: 0]
- `--state-queue-processing-length` - Processing length for state queue [default: 28807]
- `--address-queue-start-index` - Starting index for address queue [default: 0]
- `--address-queue-processing-length` - Processing length for address queue [default: 28807]

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

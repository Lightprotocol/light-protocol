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
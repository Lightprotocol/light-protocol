# @lightprotocol/program-test

LiteSVM-based testing utilities for Light Protocol programs. This is the Node.js equivalent of the `light-program-test` Rust crate.

## Features

- **LiteSVM Integration**: In-process Solana VM for fast testing without a full validator
- **TestRpc**: Mock RPC implementation that builds merkle trees from transaction events
- **Test Utilities**: Helper functions for creating test accounts and managing test state
- **Merkle Tree**: In-memory merkle tree implementation for testing

## Installation

```bash
pnpm add -D @lightprotocol/program-test
```

## Usage

### Basic Example

```typescript
import {
  createLiteSVMRpc,
  newAccountWithLamports,
} from "@lightprotocol/program-test";
import { compress, bn } from "@lightprotocol/stateless.js";
import { WasmFactory } from "@lightprotocol/hasher.rs";

// Create LiteSVM RPC instance
const lightWasm = await WasmFactory.getInstance();
const rpc = await createLiteSVMRpc(lightWasm);

// Create test account with lamports
const payer = await newAccountWithLamports(rpc, 10e9);

// Compress SOL
await compress(rpc, payer, 1e9, payer.publicKey);

// Get compressed accounts
const accounts = await rpc.getCompressedAccountsByOwner(payer.publicKey);
console.log("Compressed accounts:", accounts.items);
```

## Testing

The package includes two types of tests:

### Unit Tests (LiteSVM-based)

These tests run entirely with LiteSVM and don't require any external services:

```bash
# Run all tests (unit + e2e)
pnpm test

# Run all unit tests
pnpm test:unit:all

# Run all unit tests with V1
pnpm test:unit:all:v1

# Run all unit tests with V2
pnpm test:unit:all:v2

# Run individual unit tests
pnpm test:unit:compress      # Compression tests
pnpm test:unit:transfer      # Transfer tests
pnpm test:unit:test-rpc      # TestRpc tests

# Run all tests (no filtering)
pnpm test-all
```

**Unit test files:**

- `tests/compress.test.ts` - Compression functionality
- `tests/transfer.test.ts` - Transfer operations
- `tests/test-rpc.test.ts` - TestRpc functionality

### E2E Tests (Requires Test Validator)

These tests validate that TestRpc behavior matches the real Photon RPC by running against a test validator:

```bash
# Run all e2e tests
pnpm test:e2e:all

# Run individual e2e tests
pnpm test:e2e:rpc-interop        # RPC interoperability tests
pnpm test:e2e:rpc-multi-trees    # Multi-tree functionality tests

# Run with specific version
pnpm test:v1                     # Run all tests with V1
pnpm test:v2                     # Run all tests with V2
```

**E2E test files:**

- `tests/rpc-interop.test.ts` - Tests comparing TestRpc with real Rpc
- `tests/rpc-multi-trees.test.ts` - Tests multi-tree functionality

**Note:** E2E tests require:

1. Light Protocol programs built and deployed
2. Test validator running (started automatically via `pnpm test-validator`)
3. Photon indexer running

## API

### createLiteSVMRpc

Creates a new LiteSVM-based RPC instance for testing.

```typescript
async function createLiteSVMRpc(
  lightWasm: LightWasm,
  config?: LiteSVMConfig,
  proverEndpoint?: string,
): Promise<LiteSVMRpc>;
```

### newAccountWithLamports

Creates a new keypair and airdrops lamports to it.

```typescript
async function newAccountWithLamports(
  rpc: LiteSVMRpc,
  lamports?: number,
): Promise<Keypair>;
```

### LiteSVMRpc

Extends `TestRpc` from `@lightprotocol/stateless.js` and overrides blockchain interaction methods to use LiteSVM instead of a real validator.

Key methods:

- `sendTransaction()` - Send and execute transactions
- `getCompressedAccountsByOwner()` - Get compressed accounts by owner
- `getCompressedAccountProof()` - Get merkle proof for account
- `getValidityProof()` - Get validity proof for accounts/addresses
- All standard Solana RPC methods

## How It Works

1. **LiteSVM**: Provides an in-process Solana VM for executing transactions
2. **TestRpc**: Parses transaction events to build merkle trees in memory
3. **Proof Generation**: Generates merkle proofs from the in-memory trees
4. **No Indexer Required**: All state is maintained in memory, no Photon indexer needed for unit tests

## Development

```bash
# Build the package
pnpm build

# Run unit tests
pnpm test:unit:all

# Run e2e tests
pnpm test:e2e:all

# Run all tests (unit + e2e)
pnpm test

# Format code
pnpm format

# Lint code
pnpm lint
```

## License

Apache-2.0

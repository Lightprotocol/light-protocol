# SDK Tests - SDK Integration Test Suite

This directory contains integration tests for various SDK implementations. Tests verify that Light Protocol SDKs work correctly with Solana programs using different frameworks (native, Anchor, Pinocchio) and use cases (token operations).

## Test Organization

All tests in this directory are integration tests that run with `cargo test-sbf`, compiling programs to SBF (Solana Bytecode Format) and executing them in a Solana runtime environment.

## Environment Variables

```bash
export RUSTFLAGS="-D warnings"
export REDIS_URL=redis://localhost:6379
```

## Test Packages

V1 means v1 Merkle tree accounts (concurrent Merkle trees, state & address Merkle trees are height 26, address queue and tree are separate solana accounts).
V2 means v2 Merkle tree accounts (batched Merkle trees, state Merkle tree height 32, address Merkle tree height 40 address queue and tree are the same solana account).

### Native SDK Tests

#### V1 Native SDK
```bash
cargo test-sbf -p sdk-v1-native-test
```
Tests for Light SDK V1 with native Solana programs (without Anchor framework).

#### V2 Native SDK
```bash
cargo test-sbf -p sdk-native-test
```
Tests for Light SDK V2 with native Solana programs (without Anchor framework).

### Anchor SDK Tests

#### Rust Tests
```bash
cargo test-sbf -p sdk-anchor-test
```
Tests for Light SDK with Anchor framework (Rust tests).

#### TypeScript Tests
```bash
npx nx build @lightprotocol/sdk-anchor-test
cd sdk-tests/sdk-anchor-test
npm run test-ts
```
TypeScript integration tests for Anchor SDK.

**Test scripts** (see `sdk-tests/sdk-anchor-test/package.json`):
- `npm run build` - Build the Anchor program
- `npm run test-ts` - Run TypeScript integration tests with Light test validator

### Pinocchio SDK Tests

#### V1 Pinocchio SDK
```bash
cargo test-sbf -p sdk-pinocchio-v1-test
```
Tests for Light SDK V1 with Pinocchio framework (high-performance Solana program framework).

#### V2 Pinocchio SDK
```bash
cargo test-sbf -p sdk-pinocchio-v2-test
```
Tests for Light SDK V2 with Pinocchio framework.

### Token SDK Tests
```bash
cargo test-sbf -p sdk-token-test
```
Tests for compressed token SDK operations (mint, transfer, burn, etc.).

### Client Tests
```bash
cargo test-sbf -p client-test
```
Tests for the Light RPC client library for querying compressed accounts.

## SDK Libraries (Unit Tests)

While the above are integration tests, the actual SDK libraries are located in `sdk-libs/` and have their own unit tests:

```bash
# SDK core libraries
cargo test -p light-sdk-macros
cargo test -p light-sdk-macros --all-features
cargo test -p light-sdk
cargo test -p light-sdk --all-features

# Supporting libraries
cargo test -p light-program-test
cargo test -p light-client
cargo test -p light-sparse-merkle-tree
cargo test -p light-compressed-token-types
cargo test -p light-compressed-token-sdk
```

## Test Categories

### By Framework
- **Native**: Direct Solana program development without frameworks
- **Anchor**: Anchor framework for Solana program development
- **Pinocchio**: High-performance Solana program framework with minimal overhead

### By SDK Version
- **V1**: Original Light SDK API
- **V2**: Updated Light SDK API with improvements

### By Use Case
- **General SDK**: Core compressed account operations
- **Token SDK**: Compressed SPL token operations
- **Client SDK**: RPC client for querying compressed accounts

## CI Workflow Reference

These tests are run in the following GitHub Actions workflows:
- `.github/workflows/sdk-tests.yml` - Main SDK integration tests (also called "examples-tests")
- `.github/workflows/js-v2.yml` - TypeScript SDK tests (V2)

For the exact test matrix and execution order, see the workflow files.

## TypeScript Tests

For TypeScript/JavaScript tests in this directory, see the `package.json` files in individual test directories:
- `sdk-anchor-test/package.json` - Anchor TypeScript tests

Additional TypeScript tests for SDK libraries are located in the `js/` directory.

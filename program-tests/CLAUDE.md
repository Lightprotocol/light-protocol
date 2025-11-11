# Program Tests - Integration Test Suite

This directory contains long-running integration tests that require Solana runtime (SBF). All tests (except `zero-copy-derive-test`) depend on `program-tests/utils` (light-test-utils).

## Test Organization

### Why Tests Live Here

- **Most tests**: Depend on `program-tests/utils` (light-test-utils) for shared test infrastructure
- **`batched-merkle-tree-test`**: Specifically located here because it depends on program-tests/utils
- **`zero-copy-derive-test`**: Exception - placed here only to avoid cyclic dependencies (NOT a long-running integration test)

### Test Execution

All tests use `cargo test-sbf` which compiles programs to SBF (Solana Bytecode Format) and runs them in a Solana runtime environment.

## Environment Variables

```bash
export RUSTFLAGS="-D warnings"
export REDIS_URL=redis://localhost:6379
```

## Test Packages

### Account Compression Tests
```bash
cargo test-sbf -p account-compression-test
```
Tests for the core account compression program (Merkle tree management).

### Registry Tests
```bash
cargo test-sbf -p registry-test
```
Tests for protocol configuration and forester registration.

### Light System Program Tests

#### Address Tests
```bash
cargo test-sbf -p system-test -- test_with_address
```
Tests for address-based operations in the Light system program.

#### Compression Tests
```bash
cargo test-sbf -p system-test -- test_with_compression
cargo test-sbf -p system-test --test test_re_init_cpi_account
```
Tests for compressed account operations.

### System CPI Tests

#### V1 CPI Tests
```bash
cargo test-sbf -p system-cpi-test
```
Tests for Cross-Program Invocation (CPI) with Light system program V1.

#### V2 CPI Tests
```bash
# Main tests (excluding functional and event parsing)
cargo test-sbf -p system-cpi-v2-test -- --skip functional_ --skip event::parse

# Event parsing tests
cargo test-sbf -p system-cpi-v2-test -- event::parse

# Functional tests - read-only
cargo test-sbf -p system-cpi-v2-test -- functional_read_only

# Functional tests - account infos
cargo test-sbf -p system-cpi-v2-test -- functional_account_infos
```
Tests for Cross-Program Invocation (CPI) with Light system program V2.

### Compressed Token Tests

#### Core Token Tests
```bash
cargo test-sbf -p compressed-token-test --test ctoken
cargo test-sbf -p compressed-token-test --test v1
cargo test-sbf -p compressed-token-test --test mint
cargo test-sbf -p compressed-token-test --test transfer2
```

#### Batched Tree Tests (with retry logic in CI)
```bash
cargo test-sbf -p compressed-token-test -- test_transfer_with_photon_and_batched_tree
```
Note: CI runs this with retry logic (max 3 attempts, 5s delay) due to known flakiness.

### E2E Tests
```bash
cargo test-sbf -p e2e-test
```
End-to-end integration tests across multiple programs.

#### E2E Extended Tests
After building the small compressed token program:
```bash
pnpm --filter @lightprotocol/programs run build-compressed-token-small
cargo test-sbf -p e2e-test -- --test test_10_all
```

### Batched Merkle Tree Tests
```bash
# Skip long-running simulation and e2e tests
cargo test -p batched-merkle-tree-test -- --skip test_simulate_transactions --skip test_e2e

# Run simulation test with logging
RUST_LOG=light_prover_client=debug cargo test -p batched-merkle-tree-test -- --test test_simulate_transactions

# Run e2e test
cargo test -p batched-merkle-tree-test -- --test test_e2e
```
Note: Located in program-tests because it depends on program-tests/utils.

### Pinocchio No-std Tests
```bash
cargo test-sbf -p pinocchio-nostd-test
```
Tests for Pinocchio library in no-std environment.

### Zero-Copy Derive Tests (Unit Test Exception)
```bash
cargo test -p zero-copy-derive-test
```
**Special case**: This is NOT an integration test. It's a unit test located in program-tests only to avoid cyclic dependencies in the package dependency graph.

## Supporting Infrastructure

### Test Utilities (light-test-utils)
Located at `program-tests/utils`, this crate provides shared test infrastructure used by most integration tests.

### Test Programs
Some directories contain test programs (not tests themselves) used by the integration tests:
- `create-address-test-program`
- `merkle-tree`

## CI Workflow Reference

These tests are run in the following GitHub Actions workflows:
- `.github/workflows/programs.yml` - Main program integration tests
- `.github/workflows/rust.yml` - Batched Merkle tree tests (partial)

For the exact test matrix and execution order, see the workflow files.

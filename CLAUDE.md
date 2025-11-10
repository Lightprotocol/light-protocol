# Light Protocol - AI Assistant Reference Guide

## Repository Overview

Light Protocol is the ZK Compression Protocol for Solana, enabling developers to create rent-free tokens and PDAs without sacrificing performance, security, or composability. The protocol uses zero-knowledge proofs to compress account state into Merkle trees, reducing storage costs while maintaining full Solana compatibility.

**Core Technologies:** Rust, Solana, ZK circuits (Gnark), Poseidon hashing, batched Merkle trees
**Architecture:** On-chain programs + off-chain ZK provers + client SDKs + forester service, see light_paper_v0.1.0.pdf for details.
**Detailed docs:** See `CLAUDE.md` files in individual crates and `**/*/docs/`

## Directory Structure

```
light-protocol/
├── program-libs/              # Core Rust libraries (used in programs and sdk-libs)
│   ├── account-checks/           # Solana account validation (solana-program + pinocchio)
│   ├── aligned-sized/            # Macro to get the aligned size of rust structs
│   ├── array-map/                # Array-based map data structure
│   ├── batched-merkle-tree/      # Batched Merkle tree (Merkle tree updates with zk proofs)
│   ├── bloom-filter/             # Bloom filters
│   ├── compressed-account/       # Compressed account types and utilities
│   ├── compressible/             # Configuration for compressible token accounts
│   ├── concurrent-merkle-tree/   # Concurrent Merkle tree operations
│   ├── ctoken-types/             # Compressed token types and interfaces
│   ├── hash-set/                 # Hash set implementation for Solana programs
│   ├── hasher/                   # Poseidon hash implementation
│   ├── heap/                     # Heap data structure for Solana programs
│   ├── indexed-array/            # Indexed array utilities
│   ├── indexed-merkle-tree/      # Indexed Merkle tree with address management
│   ├── macros/                   # Procedural macros for Light Protocol
│   ├── merkle-tree-metadata/     # Metadata types for Merkle trees
│   ├── verifier/                 # ZKP verification logic in Solana programs
│   ├── zero-copy/                # Zero-copy serialization for efficient account access
│   └── zero-copy-derive/         # Derive macros for zero-copy serialization
├── programs/                  # Light Protocol Solana programs
│   ├── account-compression/      # Core compression program (owns Merkle tree accounts)
│   ├── system/                   # Light system program (compressed account validation)
│   ├── compressed-token/         # Compressed token implementation (similar to SPL Token)
│   └── registry/                 # Protocol configuration and forester access control
├── sdk-libs/                  #  Rust libraries used in custom programs and clients
│   ├── client/                   # RPC client for querying compressed accounts
│   ├── sdk/                      # Core SDK for Rust/Anchor programs
│   ├── sdk-pinocchio/            # Pinocchio-specific SDK implementation
│   ├── compressed-token-sdk/     # Compressed token client utilities
│   └── program-test/             # Fast local test environment (LiteSVM)
├── prover/                    # ZK proof generation
│   ├── server/                   # Go-based prover server and circuit implementation (Gnark)
│   └── client/                   # Rust client for requesting proofs used in sdk/client and forester
├── forester/                  # Server implementation to insert values from queue accounts into tree accounts.
├── cli/                       # Light CLI tool (@lightprotocol/zk-compression-cli)
├── js/                        # JavaScript/TypeScript libraries (@lightprotocol/stateless.js, @lightprotocol/compressed-token)
├── program-tests/             # Integration tests for programs
├── sdk-tests/                 # Integration tests for sdk libraries in solana programs that integrate light protocol.
└── scripts/                   # Build, test, and deployment scripts
```

### Program libs
- depend on other program-libs or external crates only
- unit test must not depend on light-test-utils, any test that requires light-test-utils must be in program-tests

### Programs
- depend on program-libs and external crates only
- are used in program-tests, in sdk-libs only with devenv feature but should be avoided.
- unit test must not depend on light-test-utils, any test that requires light-test-utils must be in program-tests
- integration tests must be in program-tests
- light-test-utils contains assert functions to assert instruction success in integration tests.

### SDK libs
- depend on program-libs, light-prover-client and external crates only
- must not depend on programs without devenv feature
- unit test must not depend on light-test-utils, any test that requires light-test-utils must be in sdk-tests
- integration tests must be in sdk-tests

## Development Workflow

### Build Commands
```bash
# Build entire monorepo (uses Nx)
./scripts/build.sh
```

### Testing Patterns

**IMPORTANT**: Do not run `cargo test` at the monorepo root. Always target specific packages with `-p`.

The repository has three main categories of tests:

#### 1. Unit Tests (program-libs/)
Fast-running tests that don't require Solana runtime. Located in `program-libs/` crates.

```bash
# Run with: cargo test -p <package-name>
cargo test -p light-batched-merkle-tree
cargo test -p light-account-checks
cargo test -p light-hasher --all-features
cargo test -p light-compressed-account --all-features
# ... see individual crate docs for specific tests
```

**Environment variables used in CI:**
- `RUSTFLAGS="-D warnings"` (fail on warnings)
- `REDIS_URL=redis://localhost:6379`

#### 2. Integration Tests (program-tests/)
Long-running integration tests that require Solana runtime (SBF). Located in `program-tests/`.

**Why tests live here:**
- Most depend on `program-tests/utils` (light-test-utils)
- `batched-merkle-tree-test` is here because it depends on program-tests/utils
- `zero-copy-derive-test` is here only to avoid cyclic dependencies (it's NOT a long-running integration test)

```bash
# Run with: cargo test-sbf -p <package-name>
cargo test-sbf -p account-compression-test
cargo test-sbf -p system-test
cargo test-sbf -p compressed-token-test
# ... see program-tests/CLAUDE.md for complete list
```

**For detailed test commands, see:** `program-tests/CLAUDE.md`

#### 3. SDK Tests (sdk-tests/)
SDK integration tests for various SDK implementations (native, Anchor, Pinocchio, token).

```bash
# Run with: cargo test-sbf -p <package-name>
cargo test-sbf -p sdk-native-test
cargo test-sbf -p sdk-anchor-test
cargo test-sbf -p sdk-token-test
# ... see sdk-tests/CLAUDE.md for complete list
```

**For detailed test commands, see:** `sdk-tests/CLAUDE.md`

#### 4. JavaScript/TypeScript Tests
Version-specific tests (V1 and V2) for JS/TS packages.

```bash
# Build and test with Nx
npx nx build @lightprotocol/zk-compression-cli
npx nx test @lightprotocol/stateless.js
npx nx test @lightprotocol/compressed-token
npx nx test @lightprotocol/zk-compression-cli
```

**Environment variables:**
- `LIGHT_PROTOCOL_VERSION=V1` or `V2`
- `REDIS_URL=redis://localhost:6379`
- `CI=true`

**For available test scripts, see:** `package.json` files in `js/` directory

#### 5. Go Prover Tests
Tests for the ZK proof generation server (Gnark circuits).

```bash
# Run from prover/server directory
cd prover/server

# Unit tests
go test ./prover/... -timeout 60m

# Redis integration tests
TEST_REDIS_URL=redis://localhost:6379/15 go test -v -run TestRedis -timeout 10m

# Integration tests
go test -run TestLightweight -timeout 15m
```

**For detailed test commands, see:** `prover/server/` directory

#### 6. Forester Tests
End-to-end tests for the off-chain tree maintenance service.

```bash
TEST_MODE=local cargo test --package forester e2e_test -- --nocapture
```

**Environment variables:**
- `RUST_BACKTRACE=1`
- `TEST_MODE=local`
- `REDIS_URL=redis://localhost:6379`

#### 7. Linting
Format and clippy checks across the entire codebase.

```bash
./scripts/lint.sh
```

**Note:** This requires nightly Rust toolchain and clippy components.

### Test Organization Principles

- **`program-libs/`**: Pure Rust libraries, unit tests with `cargo test`
- **`sdk-libs/`**: Pure Rust libraries, unit tests with `cargo test`
- **`program-tests/`**: Integration tests requiring Solana runtime, depend on `light-test-utils`
- **`sdk-tests/`**: SDK-specific integration tests
- **Special case**: `zero-copy-derive-test` in `program-tests/` only to break cyclic dependencies

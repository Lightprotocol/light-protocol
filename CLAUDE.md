# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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
│   ├── ctoken-interface/             # Compressed token types and interfaces
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
├── programs/                  # Light Protocol Solana programs (pinocchio-based)
│   ├── account-compression/      # Core compression program (owns Merkle tree accounts)
│   ├── system/                   # Light system program (compressed account validation)
│   ├── compressed-token/         # Compressed token implementation (similar to SPL Token)
│   └── registry/                 # Protocol configuration and forester access control
├── anchor-programs/           # Anchor-based program variants
│   └── system/                   # Anchor variant of the system program
├── sdk-libs/                  #  Rust libraries used in custom programs and clients
│   ├── client/                   # RPC client for querying compressed accounts
│   ├── sdk/                      # Core SDK for Rust/Anchor programs
│   ├── sdk-pinocchio/            # Pinocchio-specific SDK implementation
│   ├── ctoken-sdk/               # Compressed token client utilities
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

### Setup
```bash
./scripts/install.sh        # Install dependencies into .local/
./scripts/devenv.sh         # Activate development environment (uses .local/ toolchain)
solana-keygen new -o ~/.config/solana/id.json  # Generate keypair (required before testing)
```

### Build Commands
```bash
# Build with just (preferred)
just build                  # Build programs + JS + CLI
just programs::build        # Build only Solana programs

# Or with scripts
./scripts/build.sh          # Build entire monorepo (uses Nx)
```

### Format and Lint
```bash
just format                 # cargo +nightly fmt --all + JS formatting
just lint                   # Rust fmt check + clippy + README checks + JS lint
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
V1 = concurrent Merkle trees (height 26, separate queue/tree accounts). V2 = batched Merkle trees (state height 32, address height 40, combined queue/tree accounts).

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
# Build and test with just
just cli::build
just js::test-stateless
just js::test-compressed-token
just cli::test

# Or use root-level aggregates that include cli and js targets
just build
just test
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
```bash
just lint                   # Preferred: fmt check + clippy + README checks + JS lint
./scripts/lint.sh           # Alternative: shell script
```
Requires nightly Rust toolchain and clippy components.

### Test Organization Principles

- **`program-libs/`**: Pure Rust libraries, unit tests with `cargo test`
- **`sdk-libs/`**: Pure Rust libraries, unit tests with `cargo test`
- **`program-tests/`**: Integration tests requiring Solana runtime, depend on `light-test-utils`
- **`sdk-tests/`**: SDK-specific integration tests
- **Special case**: `zero-copy-derive-test` in `program-tests/` only to break cyclic dependencies

### Test Assertion Pattern

When testing account state, use borsh deserialization with a single `assert_eq` against an expected reference account:

```rust
use borsh::BorshDeserialize;
use light_ctoken_types::state::{
    AccountState, CToken, ExtensionStruct, PausableAccountExtension,
    PermanentDelegateAccountExtension,
};

// Deserialize the account
let ctoken = CToken::deserialize(&mut &account.data[..])
    .expect("Failed to deserialize CToken account");

// Extract runtime-specific values from deserialized account
let compression_info = ctoken
    .extensions
    .as_ref()
    .and_then(|exts| {
        exts.iter().find_map(|e| match e {
            ExtensionStruct::Compressible(info) => Some(info.clone()),
            _ => None,
        })
    })
    .expect("Should have Compressible extension");

// Build expected account for comparison
let expected_ctoken = CToken {
    mint: mint_pubkey.to_bytes().into(),
    owner: payer.pubkey().to_bytes().into(),
    amount: 0,
    delegate: None,
    state: AccountState::Frozen,
    is_native: None,
    delegated_amount: 0,
    close_authority: None,
    extensions: Some(vec![
        ExtensionStruct::Compressible(compression_info),
        ExtensionStruct::PausableAccount(PausableAccountExtension),
        ExtensionStruct::PermanentDelegateAccount(PermanentDelegateAccountExtension),
    ]),
};

// Single assert comparing full account state
assert_eq!(ctoken, expected_ctoken, "CToken account should match expected");
```

**Benefits:**
- Type-safe assertions using actual struct fields instead of magic byte offsets
- Maintainable - if account layout changes, deserialization handles it
- Readable - clear field names vs `account.data[108]`
- Single assertion point for the entire account state

### Getting Anchor Program Instruction Discriminators

Anchor uses 8-byte discriminators derived from the instruction name. To get discriminators from an Anchor program:

```rust
#[cfg(test)]
mod discriminator_tests {
    use super::*;
    use anchor_lang::Discriminator;

    #[test]
    fn print_instruction_discriminators() {
        // Each instruction in the #[program] module has a corresponding struct
        // in the `instruction` module with the DISCRIMINATOR constant
        println!("InstructionName: {:?}", instruction::InstructionName::DISCRIMINATOR);
    }
}
```

Run with: `cargo test -p <program-crate> print_instruction_discriminators -- --nocapture`

**Example output:**
```
Claim: [62, 198, 214, 193, 213, 159, 108, 210]
CompressAndClose: [96, 94, 135, 18, 121, 42, 213, 117]
```

**When to use discriminators:**
- Building instructions manually without Anchor's `InstructionData` trait
- Creating SDK functions that don't depend on Anchor crate
- Verifying instruction data in tests or validators

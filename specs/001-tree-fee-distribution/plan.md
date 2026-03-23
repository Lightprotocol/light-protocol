# Implementation Plan: Tree Fee Distribution

**Branch**: `jorrit/feat-specify-and-tree-fee-distribution` | **Date**: 2026-03-20 | **Spec**: `specs/001-tree-fee-distribution/spec.md`
**Input**: Feature specification from `specs/001-tree-fee-distribution/spec.md`

## Summary

Fix forester fee reimbursement and capture excess protocol fees. Three sub-features:
- **A1**: Cap address tree fee transfers at `min(5000, network_fee)` in account-compression
- **A2**: Per-tree reimbursement PDA in registry for state tree batch_nullify reimbursement
- **B**: Standalone `claim_fees` instruction in account-compression to transfer excess accumulated fees to a protocol fee recipient

## Technical Context

**Language/Version**: Rust (Solana BPF target), Anchor framework for account-compression and registry programs
**Primary Dependencies**: anchor-lang, light-batched-merkle-tree, light-merkle-tree-metadata, light-account-checks, light-compressed-account
**Storage**: Solana on-chain accounts (tree/queue accounts owned by account-compression, PDA owned by registry)
**Testing**: `cargo test-sbf -p` for integration tests in program-tests/, `cargo test -p` for unit tests in program-libs/
**Target Platform**: Solana mainnet (BPF)
**Project Type**: On-chain Solana programs (account-compression, registry)
**Performance Goals**: No additional CU overhead beyond the lamport transfers; no serialization bottlenecks (per-tree PDA, standalone claim_fees)
**Constraints**: Must not break existing batch instruction interfaces; must maintain parallelism across trees
**Scale/Scope**: All V1 and V2 tree/queue account types

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Minimalism | PASS | Each change traces to a concrete need: forester solvency (A1/A2) or protocol revenue (B). No speculative features. |
| II. Security-First | PASS | Uses `check_signer_is_registered_or_authority` for claim_fees. Checked arithmetic in excess formula. PDA seeds documented. Account validation on PDA init. |
| III. Test-Driven | PASS | Integration tests in program-tests/ with cargo test-sbf. Unit tests for fee calculation in program-libs/. |
| IV. Spec-First | PASS | Spec written and reviewed before this plan. |
| V. Explicit Over Implicit | PASS | Named error variants for insufficient PDA funds, zero claimable excess, invalid tree. Hardcoded rent exemption constants. |
| VI. Method Patterns | PASS | Anchor `#[program]` for both account-compression and registry. CPI pattern unchanged. |
| VII. Zero-Copy | PASS | claim_fees reads existing zero-copy metadata (BatchedMerkleTreeMetadata, RolloverMetadata). No new zero-copy structs needed. |
| VIII. No Unwraps | PASS | All fee arithmetic uses checked_sub/checked_mul with ? propagation. |

## Project Structure

### Documentation (this feature)

```text
specs/001-tree-fee-distribution/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0 research
├── data-model.md        # Entity definitions
└── contracts/           # Instruction interfaces
    └── instructions.md  # New/modified instruction signatures
```

### Source Code (files to modify/create)

```text
# Feature A1: Address tree fee cap
programs/account-compression/src/instructions/
├── update_address_merkle_tree.rs          # Cap network_fee transfer at min(5000, network_fee)
└── batch_update_address_tree.rs           # Cap network_fee transfer at min(5000, network_fee)

# Feature A2: Reimbursement PDA
programs/registry/src/
├── account_compression_cpi/
│   ├── batch_append.rs                    # Add forester->PDA transfer after CPI
│   ├── batch_nullify.rs                   # Add PDA->forester transfer after CPI
│   └── nullify.rs                         # Add forester->PDA clawback after CPI
├── fee_reimbursement/                     # NEW module
│   ├── mod.rs
│   ├── state.rs                           # ReimbursementPda account struct (empty, just for Anchor init)
│   └── initialize.rs                      # init_reimbursement_pda instruction
└── lib.rs                                 # Register new instructions

# Feature B: claim_fees
programs/account-compression/src/instructions/
└── claim_fees.rs                          # NEW: standalone excess fee claiming
programs/account-compression/src/lib.rs    # Register claim_fees instruction
programs/registry/src/
├── account_compression_cpi/
│   └── claim_fees.rs                      # NEW: registry wrapper for claim_fees
├── protocol_config/
│   └── state.rs                           # Rename place_holder -> protocol_fee_recipient
└── lib.rs                                 # Register claim_fees wrapper

# Fee calculation (shared)
program-libs/merkle-tree-metadata/src/
└── fee.rs                                 # Add compute_claimable_excess function + hardcoded rent constants

# Tests
program-tests/
├── account-compression-test/              # Tests for claim_fees, address tree fee cap
└── registry-test/                         # Tests for PDA init, batch_append/nullify PDA flow
```

**Structure Decision**: Changes distributed across existing crate boundaries per constitution repository structure rules. New code in programs/ depends only on program-libs/. Fee calculation logic in program-libs/merkle-tree-metadata/ (shared by account-compression for claim_fees).

## Complexity Tracking

No constitution violations to justify.

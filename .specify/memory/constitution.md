# Light Protocol Constitution

## Core Principles

### I. Minimalism

- No speculative features. Every addition MUST trace to a concrete user need or security requirement.
- Remove dead code rather than commenting it out.
- Three similar lines of code are better than a premature abstraction.
- Functions with 6+ positional parameters MUST use a named-field struct.

### II. Security-First

- All account validation MUST use `light-account-checks` (`check_owner`, `check_signer`, `check_mut`, `check_discriminator`, `check_pda_seeds`) or Anchor's `AccountLoader` (which checks owner + discriminator).
- When an instruction dispatches on runtime-determined account types: use `*_from_account_info` (light-account-checks) for V2 batched accounts and `AccountLoader::try_from` (Anchor) for V1 accounts. If Anchor lifetime constraints prevent `AccountLoader`, an explicit owner check + discriminator match is acceptable but MUST be documented with the reason.
- All arithmetic in on-chain programs MUST use checked math (`checked_add`, `checked_sub`, `checked_mul`) or equivalent safe operations.
- Merkle tree integrity: all state transitions MUST be verified by ZK proofs before any compressed account mutation.
- No upgradeable program authority without explicit constitution amendment.
- All PDAs MUST be derived deterministically with documented seeds.
- Proof verification MUST complete before any compressed account state change.

### III. Test-Driven

- Unit tests in `program-libs/` and `sdk-libs/` run with `cargo test -p <package>`.
- Integration tests in `program-tests/` run with `cargo test-sbf -p <package>`.
- SDK integration tests in `sdk-tests/` run with `cargo test-sbf -p <package>`.
- MUST NOT run `cargo test` at the monorepo root.
- Assert pattern: single `assert_eq` against a fully constructed expected struct (borsh-deserialized actual vs hand-built expected).
- Tests that depend on `light-test-utils` MUST live in `program-tests/` or `sdk-tests/`, never in `program-libs/` or `programs/`.
- When writing tests, MUST use the `/rust-test` skill for conventions: assertion patterns, assert functions per instruction, property tests with proptest, failing tests for every error variant, and Solana program testing with light-program-test.
- **Test coverage requirements**:
  - Every new instruction MUST have an integration test for the success path.
  - Every error variant MUST have a failing test that triggers it.
  - Fee/arithmetic logic MUST have unit tests covering: normal case, zero case, boundary values (off-by-one), and overflow/underflow.
  - Modified instructions MUST have tests verifying both old behavior (backward compatibility) and new behavior.
  - State transitions (PDA balance changes, account lamport changes) MUST be asserted with exact expected values, not just direction (e.g., assert `balance == 5000`, not `balance > 0`).
  - Account state MUST be asserted by deserializing the full account and comparing with a single `assert_eq` against a fully constructed expected struct -- not by checking individual fields or magic byte offsets.
  - Every new instruction MUST have a reusable assert function in `light-test-utils` (`program-tests/utils/`) that validates the instruction's effects. Tests call these assert functions rather than inlining assertions. This keeps test logic DRY and ensures consistent validation across test files.

### IV. Spec-First

- Specifications MUST be written and reviewed before implementation begins.
- Specifications are technology-agnostic: no frameworks, languages, or library names in `spec.md`. Technology choices belong in `plan.md`.
- Every feature MUST have acceptance criteria traceable from spec through plan to tasks.

### V. Explicit Over Implicit

- Every error condition MUST have a named, documented error variant.
- Constraints (size limits, valid ranges, allowed states) MUST be stated explicitly in code, not derived from context.
- No silent fallbacks or default behaviors that mask failures.

### VI. Method Patterns

- **Dual SDK Support**: Feature-gated `pinocchio` / `solana-program` via `AccountInfoTrait` abstraction. Keys as `[u8; 32]`, SDK-specific wrappers at boundaries only.
- **Instruction Dispatch**: Anchor `#[program]` for `account-compression` / `registry`; manual 8-byte discriminator dispatch for pinocchio programs (`system`).
- **Fluent CPI Builder**: `LightCpiInstruction` trait with `.new_cpi()` -> `.with_light_account()` -> `.invoke()` chaining.
- **Account Validation**: `check_owner`, `check_signer`, `check_mut`, `check_discriminator`, `check_pda_seeds` from `light-account-checks`.
- **Authority Checks**: MUST use `check_signer_is_registered_or_authority` or its `manual_*` variant via `GroupAccess` trait. Custom auth functions that mirror this logic MUST document the equivalence and the reason for not using the canonical function (e.g., Anchor lifetime constraints in dynamic dispatch).

### VII. Zero-Copy

- In-place byte access via `light-zero-copy` / `light-zero-copy-derive`.
- `#[derive(ZeroCopy)]` / `#[derive(ZeroCopyMut)]` for all on-chain account structs that require efficient access.
- Primitives MUST use little-endian wrappers (`u32` -> `U32`, `u64` -> `U64`).
- `Vec<T>` MUST be replaced with `ZeroCopySlice` in on-chain structs.
- `#[repr(C)]` MUST be present on all zero-copy structs.
- Heap allocations are forbidden in on-chain program code.

### VIII. No Unwraps Outside Tests

- Production code (non-`#[cfg(test)]`) MUST use `?` propagation or explicit error handling.
- `.unwrap()` and `.expect()` are permitted only in test code and build scripts.

## Repository Structure

Dependency rules enforced across the monorepo:

- `program-libs/` depends on other `program-libs/` crates and external crates only.
- `programs/` depends on `program-libs/` and external crates only.
- `sdk-libs/` depends on `program-libs/` and external crates only (no `programs/` dependency without `devenv` feature).
- Integration tests live in `program-tests/` (depend on `light-test-utils`) and `sdk-tests/`.

## Specification Structure

Specifications follow the nested directory convention:

```
specs/
  NNN-feature-name/
    spec.md           # Technology-agnostic requirements
    plan.md           # Technical implementation plan
    tasks.md          # Ordered task breakdown
    research.md       # Phase 0 research decisions
    data-model.md     # Entity definitions
    contracts/        # Interface contracts
    checklists/       # Quality checklists
```

## Branch Naming

Branches MUST follow `<author>/<type>-<description>` where:

- `<author>` is the developer's name (e.g., `jorrit`)
- `<type>` is one of: `feat`, `fix`, `chore`, `docs`, `experiment`
- `<description>` is kebab-case summarizing the change

Examples: `jorrit/feat-compressible-mint`, `jorrit/fix-macro-deps`, `jorrit/chore-bump-pinocchio`.

## Governance

- This constitution supersedes all conflicting practices. MUST-level principles are enforced as CRITICAL findings by `/speckit.analyze`.
- Amendments require: (1) written proposal, (2) explicit approval, (3) updated version below.
- All code reviews MUST verify compliance with these principles.

**Version**: 1.1.0 | **Ratified**: 2026-03-20 | **Last Amended**: 2026-03-21

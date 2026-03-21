# Tasks: Tree Fee Distribution

**Input**: Design documents from `specs/001-tree-fee-distribution/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/instructions.md

**Tests**: Integration tests included per test organization rules (program-tests/ with cargo test-sbf).

**Organization**: Tasks grouped by user story. US6+US7 (A1) are independent. US8/US1/US2/US5 (A2) depend on each other. US4/US3 (B) form a separate chain.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story (US1-US8)
- Exact file paths included

---

## Phase 1: Setup

**Purpose**: Shared constants, error variants, and foundational changes

- [ ] T001 Add `FORESTER_REIMBURSEMENT_CAP` constant (5000u64) in `program-libs/merkle-tree-metadata/src/fee.rs`
- [ ] T002 [P] Add hardcoded rent exemption constants and `data_len()`-to-rent lookup function in `program-libs/merkle-tree-metadata/src/fee.rs`. Enumerate all deployed V1 tree configurations and V2 account sizes.
- [ ] T003 [P] Add `compute_claimable_excess` function in `program-libs/merkle-tree-metadata/src/fee.rs`. Takes `account_lamports`, `rent_exemption`, `rollover_fee`, `capacity`, `next_index`. Returns `Option<u64>` (None if no excess). All checked arithmetic.
- [ ] T004 [P] Add error variants to `programs/account-compression/src/errors.rs`: `InvalidAccountType`, `NoExcessFees`
- [ ] T005 [P] Add error variants to `programs/registry/src/errors.rs`: `InvalidFeeRecipient`, `InvalidTreeForReimbursementPda`
- [ ] T006 [P] Add unit tests for `compute_claimable_excess` in `program-libs/merkle-tree-metadata/src/fee.rs` covering: normal excess, zero excess, negative excess (underflow), off-by-one at `next_index == 0` and `next_index == capacity`

**Checkpoint**: Constants and shared logic ready for all features.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: ProtocolConfig field rename that all features reference

- [ ] T007 Rename `place_holder: Pubkey` to `protocol_fee_recipient: Pubkey` in `programs/registry/src/protocol_config/state.rs`. Update Default and testnet_default impls. Update all references across the registry crate.

**Checkpoint**: ProtocolConfig ready. User story implementation can begin.

---

## Phase 3: Address Tree Fee Cap (US6 + US7, Priority: P1)

**Goal**: Cap address tree fee transfers at `min(5000, network_fee)` in account-compression. Simplest change, no PDA involvement.

**Independent Test**: Call address tree operations with `network_fee == 10000` and verify forester receives exactly 5000.

- [ ] T008 [P] [US6] Cap fee transfer in `programs/account-compression/src/instructions/update_address_merkle_tree.rs`: replace `network_fee` with `network_fee.min(FORESTER_REIMBURSEMENT_CAP)` in the transfer_lamports call
- [ ] T009 [P] [US7] Cap fee transfer in `programs/account-compression/src/instructions/batch_update_address_tree.rs`: replace `network_fee` with `network_fee.min(FORESTER_REIMBURSEMENT_CAP)` in the transfer_lamports call
- [ ] T010 [US6] [US7] Add integration tests in `program-tests/account-compression-test/` for address tree fee cap: test with `network_fee == 10000` (capped to 5000), `network_fee == 5000` (unchanged), `network_fee == 0` (no transfer)

**Checkpoint**: Address tree over-reimbursement fixed. US6 and US7 independently testable.

---

## Phase 4: Reimbursement PDA Init (US8, Priority: P1)

**Goal**: Create the per-tree reimbursement PDA on the registry program. Prerequisite for US1, US2, US5.

**Independent Test**: Call init_reimbursement_pda for a valid state tree and verify PDA exists with rent-exempt balance.

- [ ] T011 [US8] Create `programs/registry/src/fee_reimbursement/mod.rs` module with `state.rs` and `initialize.rs`
- [ ] T012 [US8] Define `ReimbursementPda` account struct (empty, discriminator only) in `programs/registry/src/fee_reimbursement/state.rs`
- [ ] T013 [US8] Implement `init_reimbursement_pda` instruction in `programs/registry/src/fee_reimbursement/initialize.rs`: validate tree owned by account-compression, validate state tree discriminator (V1 StateMerkleTreeAccount or V2 StateV2), create PDA with seeds `[b"reimbursement", tree.key().as_ref()]`
- [ ] T014 [US8] Register `init_reimbursement_pda` instruction in `programs/registry/src/lib.rs`
- [ ] T015 [US8] Add integration tests in `program-tests/registry-test/` for PDA init: success, double-init fails, invalid tree fails, address tree rejected, network_fee == 0 tree still allows PDA init

**Checkpoint**: PDA initialization ready. US1, US2, US5 can proceed.

---

## Phase 5: Batch Append Funds PDA + Batch Nullify Reimbursement (US1 + US2, Priority: P1)

**Goal**: batch_append deposits `network_fee` into PDA. batch_nullify disburses 5000 from PDA to forester. These are tightly coupled and must be tested together.

**Independent Test**: After 1 batch_append + 1 batch_nullify, forester's net SOL change is >= 0.

- [ ] T016 [US2] Modify `programs/registry/src/account_compression_cpi/batch_append.rs`: add `reimbursement_pda` (mut) and `system_program` accounts. After CPI, if `network_fee >= 5000`: `system_program::transfer(authority, reimbursement_pda, network_fee)`.
- [ ] T017 [US1] Modify `programs/registry/src/account_compression_cpi/batch_nullify.rs`: add `reimbursement_pda` (mut) account. Change `authority` to `#[account(mut)]`. After CPI, if PDA lamports >= rent_exempt + 5000: direct lamport transfer from PDA to authority (5000).
- [ ] T018 [US1] [US2] Update registry `lib.rs` instruction handlers to pass new accounts through for batch_append and batch_nullify
- [ ] T019 [US1] [US2] Add integration tests in `program-tests/registry-test/`: batch_append funds PDA (assert PDA +5000, forester net +5000), batch_nullify disburses (assert forester +5000, PDA -5000), batch_nullify with empty PDA (succeeds, forester +0), network_fee == 0 skips everything

**Checkpoint**: Forester solvency for batched state tree operations. SC-001 verifiable.

---

## Phase 6: Non-Batched Nullify Cap (US5, Priority: P1)

**Goal**: nullify_leaves registry wrapper claws back `network_fee - 5000` from forester to PDA.

**Independent Test**: Call nullify_leaves with `network_fee == 10000`, verify forester nets 5000.

- [ ] T020 [US5] Modify `programs/registry/src/account_compression_cpi/nullify.rs`: add `reimbursement_pda` (mut) and `system_program` accounts. After CPI, if `network_fee > 5000`: `system_program::transfer(authority, reimbursement_pda, network_fee - 5000)`.
- [ ] T021 [US5] Update registry `lib.rs` instruction handler for nullify to pass new accounts
- [ ] T022 [US5] Add integration tests in `program-tests/registry-test/`: nullify_leaves with `network_fee == 5000` (no clawback), `network_fee == 10000` (clawback 5000 to PDA), `network_fee < 5000` (no clawback), `network_fee == 0` (no transfer, no PDA interaction)

**Checkpoint**: All P1 forester reimbursement features complete.

---

## Phase 7: Protocol Fee Recipient Config (US4, Priority: P2)

**Goal**: Protocol authority can set the fee recipient address.

**Independent Test**: Call update_protocol_config, verify protocol_fee_recipient is stored.

- [ ] T023 [US4] Verify `update_protocol_config` handles the renamed `protocol_fee_recipient` field correctly (should work automatically since the full struct is replaced). Add a test if not already covered.
- [ ] T024 [US4] Add integration test in `program-tests/registry-test/`: set protocol_fee_recipient, read back, assert match. Test non-authority signer fails.

**Checkpoint**: Protocol config ready for claim_fees.

---

## Phase 8: Claim Fees (US3, Priority: P2)

**Goal**: Standalone claim_fees instruction transfers excess accumulated fees from tree/queue accounts to protocol fee recipient.

**Independent Test**: Accumulate fees in a tree, call claim_fees, verify tree balance matches formula and fee_recipient received excess.

- [ ] T025 [US3] Implement `claim_fees` instruction in `programs/account-compression/src/instructions/claim_fees.rs`: accounts (authority, registered_program_pda, merkle_tree_or_queue, fee_recipient). Determine account type from discriminator. Read rollover_fee/capacity/next_index per FR-016 table. Look up hardcoded rent for account size. Compute claimable excess. Transfer if positive.
- [ ] T026 [US3] Register `claim_fees` in `programs/account-compression/src/lib.rs` and add to instruction enum
- [ ] T027 [US3] Implement registry wrapper `claim_fees` in `programs/registry/src/account_compression_cpi/claim_fees.rs`: validate forester, verify fee_recipient matches protocol_config.protocol_fee_recipient, CPI to account-compression claim_fees
- [ ] T028 [US3] Register registry `claim_fees` wrapper in `programs/registry/src/lib.rs`
- [ ] T029 [US3] Add integration tests in `program-tests/account-compression-test/`: claim_fees on V2 state tree, V2 output queue, V2 address tree, V1 state tree, V1 address tree. Assert balance matches `rent + rollover_fee * (capacity - next_index + 1)`.
- [ ] T030 [US3] Add negative integration tests in `program-tests/registry-test/`: unregistered forester fails, fee_recipient mismatch fails, excluded account types fail (V1 nullifier queue, V1 address queue), zero excess is no-op

**Checkpoint**: Feature B complete. SC-002 and SC-003 verifiable.

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Validation, review, and cleanup

- [ ] T031 Run `/logic-review` on fee arithmetic in a loop (5 iterations), fixing all issues found each iteration. Target files: `program-libs/merkle-tree-metadata/src/fee.rs`, `programs/account-compression/src/instructions/claim_fees.rs`, `programs/registry/src/fee_reimbursement/`, and all modified batch/nullify files.
- [ ] T032 Run existing integration test suites to verify backward compatibility: `cargo test-sbf -p account-compression-test`, `cargo test-sbf -p system-test`, `cargo test-sbf -p registry-test`
- [ ] T033 Verify all quickstart.md scenarios pass end-to-end
- [ ] T034 Run `./scripts/lint.sh` and fix all reported issues

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies -- start immediately
- **Phase 2 (Foundational)**: Depends on T001 (constant)
- **Phase 3 (A1 Address Cap)**: Depends on Phase 1 (T001 constant). Independent of Phase 2.
- **Phase 4 (PDA Init)**: Depends on Phase 2 (T007 renamed field for future phases)
- **Phase 5 (Batch Flow)**: Depends on Phase 4 (PDA exists)
- **Phase 6 (Nullify Cap)**: Depends on Phase 4 (PDA exists)
- **Phase 7 (Fee Recipient)**: Depends on Phase 2 (T007 renamed field)
- **Phase 8 (Claim Fees)**: Depends on Phase 1 (T002, T003) + Phase 7 (T023)
- **Phase 9 (Polish)**: Depends on all previous phases

### User Story Dependencies

- **US6, US7** (A1 address cap): Independent of all other stories
- **US8** (PDA init): Independent, but prerequisite for US1, US2, US5
- **US1, US2** (batch PDA flow): Depend on US8
- **US5** (nullify cap): Depends on US8
- **US4** (fee recipient config): Independent
- **US3** (claim_fees): Depends on US4

### Parallel Opportunities

- T002, T003, T004, T005, T006 (Phase 1) all in parallel
- T008, T009 (Phase 3) in parallel
- Phase 3 (A1) and Phase 4 (PDA init) can run in parallel
- T016, T017 (Phase 5) can start in parallel (different files)
- Phase 7 (US4) can run in parallel with Phases 4-6

---

## Implementation Strategy

### MVP First (P1 Stories)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (rename field)
3. Complete Phase 3: Address tree fee cap (US6 + US7) -- quickest win
4. Complete Phase 4: PDA init (US8) -- unblocks remaining P1 stories
5. Complete Phase 5: Batch flow (US1 + US2) -- core forester solvency fix
6. Complete Phase 6: Nullify cap (US5) -- completes all P1 stories
7. **STOP and VALIDATE**: All forester reimbursement features working

### P2 Features (after MVP validated)

8. Complete Phase 7: Fee recipient config (US4)
9. Complete Phase 8: Claim fees (US3)
10. Complete Phase 9: Polish

### Total Tasks: 34

| Phase | Stories | Tasks |
|-------|---------|-------|
| 1: Setup | -- | 6 |
| 2: Foundational | -- | 1 |
| 3: A1 Address Cap | US6, US7 | 3 |
| 4: PDA Init | US8 | 5 |
| 5: Batch Flow | US1, US2 | 4 |
| 6: Nullify Cap | US5 | 3 |
| 7: Fee Recipient | US4 | 2 |
| 8: Claim Fees | US3 | 6 |
| 9: Polish | -- | 4 |

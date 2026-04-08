# Feature Specification: Tree Fee Distribution

**Feature Branch**: `jorrit/feat-specify-and-tree-fee-distribution`
**Created**: 2026-03-20
**Status**: Draft
**Input**: Fix forester fee reimbursement and capture excess protocol fees

## Design Constraint

Two types of fee transfers, each with different ownership constraints:

**Feature A -- Forester reimbursement (5000 lamports per operation):** Two sub-features:
- **A1 (address trees):** Account-compression caps fee transfer at `min(5000, network_fee)` directly. No PDA or registry wrapper changes needed.
- **A2 (state trees):** Reimbursement PDA in registry wrappers. batch_append funds PDA, batch_nullify disburses from PDA, nullify_leaves claws back excess to PDA. Account-compression unchanged for state tree batch instructions.

**Feature B -- Excess fee claiming from tree/queue accounts:** A standalone `claim_fees` instruction, NOT part of the forester batch flow. Accumulated fees sit in accounts owned by the account-compression program, so account-compression provides the instruction. Any registered forester can call it, but fees go to the `protocol_fee_recipient` defined in ProtocolConfig. Decoupled from batch operations to avoid serializing all forester transactions on the global fee recipient account.

## User Scenarios & Testing

### User Story 1 - Forester Stays Solvent During Nullify Operations (Priority: P1)

A forester performing nullify operations on batched state trees is reimbursed for transaction costs, preventing balance drain. Currently, foresters pay Solana transaction fees (5000 lamports) per nullify batch but receive no reimbursement. Over time this drains the forester's SOL balance and halts tree maintenance.

**Why this priority**: Without reimbursement, foresters stop operating, which blocks all nullification and eventually halts the protocol.

**Independent Test**: Run a forester that performs batch_nullify operations and verify its SOL balance does not decrease beyond Solana tx fees when a reimbursement PDA is funded.

**Acceptance Scenarios**:

1. **Given** a reimbursement PDA with sufficient funds, **When** a forester calls batch_nullify via registry, **Then** the forester receives 5000 lamports from the PDA.
2. **Given** a reimbursement PDA with 0 lamports, **When** a forester calls batch_nullify via registry, **Then** the nullification still succeeds but no reimbursement is transferred.
3. **Given** a tree with `network_fee == 0` (private tree), **When** a forester calls batch_nullify, **Then** no reimbursement is attempted.

---

### User Story 2 - Batch Append Funds the Reimbursement PDA (Priority: P1)

When a forester processes a batch append, account-compression transfers `network_fee * 2` (10,000 lamports) to the forester as before. The registry wrapper then transfers `network_fee` (5000) from the forester to the reimbursement PDA, pre-funding the next nullify reimbursement.

**Why this priority**: This is the funding mechanism for User Story 1. Without it, the reimbursement PDA has no funds.

**Independent Test**: Call batch_append via registry and verify forester net receives 5000 lamports and reimbursement PDA receives 5000 lamports.

**Acceptance Scenarios**:

1. **Given** a batched state tree with `network_fee >= 5000`, **When** a forester calls batch_append via registry, **Then** account-compression transfers `network_fee * 2` to forester, then registry transfers `network_fee` from forester to reimbursement PDA. Net forester gain: `network_fee`.
2. **Given** a tree with `network_fee == 0`, **When** a forester calls batch_append, **Then** no transfers occur (same as current behavior).

---

### User Story 3 - Protocol Captures Excess Fees (Priority: P2)

A standalone `claim_fees` instruction transfers accumulated excess fees from a tree/queue account to the protocol fee recipient defined in ProtocolConfig. Any registered forester can execute this instruction, but the fees always go to the configured fee recipient (not the forester). This is decoupled from the forester batch flow to avoid serializing all forester transactions on a single global account.

**Why this priority**: Recovers protocol revenue. Not blocking like P1, but financially significant over time.

**Independent Test**: Accumulate network fees via user transactions, then call claim_fees and verify excess flows to the protocol fee recipient.

**Acceptance Scenarios**:

1. **Given** a tree account with accumulated fees exceeding rent + rollover reserves, **When** a registered forester calls claim_fees, **Then** the excess is transferred to the protocol fee recipient.
2. **Given** an unregistered signer, **When** claim_fees is called, **Then** the transaction fails.
3. **Given** a tree with no excess fees, **When** claim_fees is called, **Then** no transfer occurs.

---

### User Story 4 - Protocol Authority Configures Fee Recipient (Priority: P2)

The protocol authority sets and updates the fee recipient address in protocol configuration.

**Why this priority**: Prerequisite for User Story 3.

**Independent Test**: Call update_protocol_config with a new fee recipient address and verify it is stored.

**Acceptance Scenarios**:

1. **Given** protocol authority, **When** update_protocol_config is called with a fee recipient address, **Then** the address is stored in ProtocolConfig.
2. **Given** a non-authority signer, **When** update_protocol_config is called, **Then** the transaction fails.

---

### User Story 5 - Non-Batched Nullify Caps Reimbursement (Priority: P1)

A forester calling the non-batched nullify_leaves instruction receives the full `network_fee` from account-compression (unchanged). The registry wrapper then transfers the excess (`network_fee - 5000`) from the forester back to the reimbursement PDA, capping the forester's net gain at 5000 lamports.

**Why this priority**: Prevents over-reimbursement that drains tree accounts when foresters batch multiple nullify_leaves calls.

**Independent Test**: Call nullify_leaves via registry and verify forester net receives exactly 5000 lamports, with any remainder going to the reimbursement PDA.

**Acceptance Scenarios**:

1. **Given** a tree with `network_fee == 5000`, **When** a forester calls nullify_leaves via registry, **Then** account-compression transfers 5000 to forester, registry transfers 0 (no excess). Net: 5000.
2. **Given** a tree with `network_fee == 10000`, **When** a forester calls nullify_leaves via registry, **Then** account-compression transfers 10000 to forester, registry transfers 5000 from forester to PDA. Net: 5000.
3. **Given** a tree with `network_fee == 5000` and two nullify_leaves calls in one transaction, **Then** forester nets 5000 per call (10000 total). Per-tx deduplication is not feasible on-chain; the 5000 cap per instruction is sufficient.

---

### User Story 6 - Non-Batched Address Tree Update Caps Reimbursement (Priority: P1)

Account-compression caps the fee transfer for update_address_merkle_tree at `min(5000, network_fee)` directly. No registry wrapper changes or PDA involvement needed.

**Why this priority**: Prevents over-reimbursement that drains address tree accounts.

**Independent Test**: Call update_address_merkle_tree and verify forester receives exactly `min(5000, network_fee)`.

**Acceptance Scenarios**:

1. **Given** an address tree with `network_fee == 10000`, **When** a forester calls update_address_merkle_tree, **Then** forester receives 5000 (capped).
2. **Given** an address tree with `network_fee == 5000`, **When** a forester calls update_address_merkle_tree, **Then** forester receives 5000.
3. **Given** an address tree with `network_fee == 0`, **When** a forester calls update_address_merkle_tree, **Then** no transfer occurs.

---

### User Story 7 - Batch Address Tree Update Caps Reimbursement (Priority: P1)

Account-compression caps the fee transfer for batch_update_address_tree at `min(5000, network_fee)` directly. No registry wrapper changes or PDA involvement needed.

**Why this priority**: Prevents over-reimbursement on batched address tree operations.

**Independent Test**: Call batch_update_address_tree and verify forester receives exactly `min(5000, network_fee)`.

**Acceptance Scenarios**:

1. **Given** an address tree with `network_fee == 10000`, **When** a forester calls batch_update_address_tree, **Then** forester receives 5000 (capped).
2. **Given** an address tree with `network_fee == 5000`, **When** a forester calls batch_update_address_tree, **Then** forester receives 5000.
3. **Given** an address tree with `network_fee == 0`, **When** a forester calls batch_update_address_tree, **Then** no transfer occurs.

---

### User Story 8 - Initialize Reimbursement PDA (Priority: P1)

A reimbursement PDA must be created for a state tree before the registry wrappers can fund or disburse from it. Anyone can call the init instruction, but it validates the tree is a real account-compression tree.

**Why this priority**: Prerequisite for User Stories 1 and 2. Without the PDA, registry wrappers cannot transfer to/from it.

**Independent Test**: Call the init instruction for a valid tree and verify the PDA is created.

**Acceptance Scenarios**:

1. **Given** a valid state tree owned by account-compression, **When** init_reimbursement_pda is called, **Then** the PDA is created with seeds `[b"reimbursement", tree_pubkey]` and rent-exempt balance.
2. **Given** an already-initialized PDA for a tree, **When** init_reimbursement_pda is called again, **Then** the transaction fails (Anchor init constraint).
3. **Given** an account not owned by account-compression, **When** init_reimbursement_pda is called with it as the tree, **Then** the transaction fails.
4. **Given** an address tree (not a state tree), **When** init_reimbursement_pda is called, **Then** the transaction fails (PDA is only for state trees).

---

### Edge Cases

- What happens when the reimbursement PDA has less than 5000 lamports beyond rent exemption? No reimbursement. Operation still succeeds.
- What happens when claimable excess is negative (tree underfunded)? No excess transfer; tree retains all lamports.
- What happens during tree rollover? Out of scope for this spec. PDA lifecycle during rollover (migrating funds, creating PDA for new tree) will be addressed in a follow-up PR.
- What happens if batch_append and batch_nullify run concurrently on the same tree? Each operates on its own account (output queue vs merkle tree); the per-tree PDA is serialized by Solana runtime when both reference the same tree.
- What happens to existing trees with accumulated excess fees? They are claimed when claim_fees is called after the upgrade. No migration needed.
- What happens when nullify_leaves is called with network_fee < 5000? Full network_fee goes to forester, nothing to PDA.
- What happens if a forester calls account-compression directly (bypassing registry)? Fee distribution only applies through registry wrappers. Direct calls get the old behavior.
- What about forester client (forester/) changes? Deferred to a follow-up PR. The forester service will need to: call init_reimbursement_pda for state trees, pass reimbursement_pda in registry wrapper calls, and periodically call claim_fees.
- What happens when the reimbursement PDA does not exist yet? It must be created before the first batch operation that needs it. Creation can be a separate instruction or lazy-initialized on first use.

## Requirements

### Functional Requirements

#### Feature A1: Address Tree Fee Cap (account-compression change)

- **FR-001**: For address tree operations (V1 `update_address_merkle_tree`, V2 `batch_update_address_tree`), account-compression MUST cap the fee transfer to the forester at `min(5000, network_fee)` instead of the full `network_fee`.
- **FR-002**: The 5000 lamport cap is a hardcoded protocol constant.

#### Feature A2: State Tree Reimbursement PDA (registry wrappers)

- **FR-003**: System MUST maintain a per-tree reimbursement escrow account (PDA) on the registry program that holds lamports for batch_nullify reimbursements.
- **FR-004**: The reimbursement PDA MUST be derived from seeds `[b"reimbursement", tree_pubkey.as_ref()]` where `tree_pubkey` is always the merkle tree account pubkey. One PDA per tree avoids global contention.
- **FR-005**: On batch_append, the registry wrapper MUST transfer `FORESTER_REIMBURSEMENT_CAP` lamports (5000) from the forester to the tree's reimbursement PDA (after account-compression has transferred `network_fee * 2` to the forester). This only applies when `network_fee >= FORESTER_REIMBURSEMENT_CAP`. Net forester gain: `network_fee * 2 - FORESTER_REIMBURSEMENT_CAP`.
- **FR-006**: On batch_nullify, the registry wrapper MUST transfer 5000 lamports from the tree's reimbursement PDA to the forester, if the PDA has sufficient funds.
- **FR-007**: If the reimbursement PDA has insufficient funds, batch_nullify MUST still succeed without reimbursement.
- **FR-008**: All trees (state and address) with `network_fee == 0` MUST skip all fee transfers, reimbursement, and PDA logic. No reimbursement without a network fee.
- **FR-009**: On nullify_leaves (non-batched), the registry wrapper MUST transfer `network_fee - 5000` lamports from the forester to the tree's reimbursement PDA (after account-compression has transferred `network_fee` to the forester). If `network_fee <= 5000`, no transfer occurs.
- **FR-010**: The reimbursement PDA MUST be initialized via a standalone instruction before it can receive funds. The instruction MUST validate that the provided tree account is a valid tree (owned by account-compression, correct discriminator) before creating the PDA.

#### Feature B: Excess Fee Claiming (standalone instruction)

- **FR-011**: ProtocolConfig MUST expose a `protocol_fee_recipient` address field (repurposing existing `place_holder: Pubkey`).
- **FR-012**: The protocol fee recipient MUST be updatable via the existing update_protocol_config instruction by the protocol authority.
- **FR-013**: Account-compression MUST provide a new `claim_fees` instruction that transfers excess fees from a tree or queue account to a `fee_recipient` account. This instruction is NOT part of any batch operation. It MUST follow the existing account-compression security model: `check_signer_is_registered_or_authority` (registered program PDA or tree owner).
- **FR-014**: The registry MUST wrap `claim_fees` with forester eligibility validation and pass the `protocol_fee_recipient` from ProtocolConfig as the `fee_recipient`. Only registered foresters may call the registry wrapper; fees always go to the configured fee recipient, not to the forester.
- **FR-015**: Claimable excess MUST be calculated as: `account_lamports - hardcoded_rent_exemption - rollover_fee * (capacity - next_index + 1)`. The first leaf (index 0) does not pay a rollover fee, so paid fees = `next_index - 1` and remaining = `capacity - next_index + 1`. Field mapping per account type:

  | Account Type | capacity | next_index |
  |---|---|---|
  | V2 state tree (batched) | `BatchedMerkleTreeMetadata.capacity` | `BatchedMerkleTreeMetadata.next_index` |
  | V2 output queue | `BatchedQueueMetadata.tree_capacity` | `BatchedQueueMetadata.batch_metadata.next_index` |
  | V2 address tree (batched) | `BatchedMerkleTreeMetadata.capacity` | `BatchedMerkleTreeMetadata.next_index` |
  | V1 state merkle tree | `2^height` (derived from `ConcurrentMerkleTree`) | `ConcurrentMerkleTree.next_index()` |
  | V1 address merkle tree | `2^height` (derived from `IndexedMerkleTree`) | `IndexedMerkleTree.next_index()` |

  Note: paid rollover fees correspond to `next_index - 1` (the first leaf at index 0 does not pay a rollover fee). All arithmetic in the excess formula MUST use checked operations (`checked_sub`, `checked_mul`) per constitution principle II.
- **FR-016**: The claim_fees instruction MUST support all tree and queue account types that accumulate fees, across both V1 and V2:
  - V2 state tree merkle tree account (input queue embedded)
  - V2 output queue account (append fees)
  - V2 address tree merkle tree account
  - V1 state merkle tree account
  - V1 address merkle tree account
  - V1 nullifier queues are excluded: `rollover_fee` is 0, no fees accumulate (fees charged on merkle tree side).
  - V1 address queues are excluded: allocated with exactly rent-exempt lamports, no fees accumulate (network fees are transferred out during updates).
- **FR-017**: Rent exemption values MUST be hardcoded constants per account type/size (not queried at runtime), because the Solana rent formula may change after trees are initialized. Constants are derived using `solana rent <account_size_bytes>` for each account type at deployment time. Different account types (state tree, output queue, address tree) have different sizes and therefore different constants.
- **FR-018**: If claimable excess is zero or negative, no transfer MUST occur.
- **FR-019**: Both `rollover_fee` and `network_fee` are read from the account's own `RolloverMetadata`. `network_fee` is always stored on the merkle tree metadata for all versions. `rollover_fee` differs by version:
  - V2 state trees: merkle tree `rollover_fee` is 0; all rollover fees are on the output queue.
  - V1 state trees: merkle tree `rollover_fee` is non-zero (covers both tree + queue rent); the associated nullifier queue has `rollover_fee` = 0.
  - V1/V2 address trees: merkle tree `rollover_fee` is non-zero.

#### Shared

- **FR-020**: Existing trees MUST NOT require migration. Fee claiming reads from existing on-chain metadata.
- **FR-021**: State tree batch instructions (batch_append, batch_nullify) in account-compression are unchanged. Registry wrappers handle PDA logic.
- **FR-022**: Address tree instructions in account-compression are modified only to cap fee transfer at `min(5000, network_fee)`.

### Key Entities

- **Reimbursement PDA**: Per-tree registry program PDA (seeds: `[b"reimbursement", tree_pubkey]`) that escrows lamports. Funded by batch_append and non-batched instruction excess; disbursed to forester on batch_nullify.
- **Protocol Fee Recipient**: Address in ProtocolConfig (repurposed `place_holder` field) that receives excess accumulated fees from tree/queue accounts.
- **Claimable Excess**: The lamports in a tree/queue account beyond what is needed for rent exemption and remaining rollover fee capacity. Calculated per account, after existing forester transfers.

## Success Criteria

### Measurable Outcomes

- **SC-001**: After 1 batch_append and 1 batch_nullify on a funded state tree, the forester's net SOL change is >= 0 (reimbursements cover tx fees).
- **SC-002**: After claim_fees is called on a tree with accumulated fees, the account balance equals exactly `hardcoded_rent_exemption + rollover_fee * (capacity - next_index + 1)`.
- **SC-003**: Protocol fee recipient account balance increases by the expected excess after claim_fees.
- **SC-004**: All existing integration tests pass without modification (batch instructions unchanged).
- **SC-005**: Account-compression changes are limited to the new claim_fees instruction and the address tree fee cap (`min(5000, network_fee)`).

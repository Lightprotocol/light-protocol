# Research: Tree Fee Distribution

## R1: Where to implement fee cap for address trees

**Decision**: Directly in account-compression program (modify existing transfer logic)
**Rationale**: Address tree instructions already transfer `network_fee` in account-compression. Capping at `min(5000, network_fee)` is a one-line change per instruction. No PDA or registry wrapper changes needed.
**Alternatives considered**:
- Registry wrapper clawback (rejected: unnecessary complexity, extra transfer, PDA contention for address trees)

## R2: Where to implement reimbursement PDA for state trees

**Decision**: Registry program owns the PDA; registry wrappers handle all transfers
**Rationale**: Account-compression batch instructions are unchanged. The PDA is a registry concern (forester access control). Per-tree PDA seeds `[b"reimbursement", tree_pubkey]` avoid global contention.
**Alternatives considered**:
- Global PDA (rejected: serializes all forester transactions)
- Account-compression owns PDA (rejected: mixes fee distribution with tree management)
- Modify batch_append to split transfer (rejected: changes account-compression interface)

## R3: Where to implement claim_fees

**Decision**: New instruction in account-compression, wrapped by registry
**Rationale**: Only account-compression can debit its own accounts (tree/queue). Registry wrapper adds forester validation and resolves fee_recipient from ProtocolConfig.
**Alternatives considered**:
- Registry-only instruction (rejected: cannot debit account-compression-owned accounts)
- Inline in batch operations (rejected: writes to global fee_recipient account, kills parallelism)

## R4: Protocol fee recipient storage

**Decision**: Repurpose `place_holder: Pubkey` in ProtocolConfig
**Rationale**: Existing unused Pubkey field, same size, no account layout change. Already updatable via `update_protocol_config`.
**Alternatives considered**:
- New field from u64 placeholders (rejected: more complex, needs two u64s for one Pubkey)
- Separate PDA (rejected: unnecessary indirection)

## R5: Fee formula: `capacity - next_index + 1`

**Decision**: `claimable = account_lamports - hardcoded_rent_exemption - rollover_fee * (capacity - next_index + 1)`
**Rationale**: First leaf (index 0) does not pay a rollover fee. So paid fees = `next_index - 1`, remaining unfunded = `capacity - (next_index - 1)` = `capacity - next_index + 1`.
**Alternatives considered**:
- `capacity - next_index` (rejected: off by one, would over-claim by one rollover_fee)

## R6: Hardcoded rent exemption

**Decision**: Per-account-type constants derived from `solana rent <size>` at deployment time
**Rationale**: Solana rent formula may change. Trees initialized today must retain correct reserves regardless of future rent changes.
**Alternatives considered**:
- Query Sysvar::Rent at runtime (rejected: rent could change, draining tree reserves)
- Store rent in tree metadata at init time (rejected: requires migration for existing trees)

## R7: V1 nullifier queue and address queue exclusion

**Decision**: Exclude both from claim_fees
**Rationale**: V1 nullifier queues have `rollover_fee = 0` (fees charged on merkle tree side). V1 address queues are allocated with exactly rent-exempt lamports; network fees are transferred out during updates. Neither accumulates excess fees.
**Alternatives considered**:
- Include with degenerate formula (rejected: no fees to claim, wastes CU)

## R8: V1 vs V2 state tree rollover_fee differences

**Decision**: Read `rollover_fee` from account's own `RolloverMetadata`; no special-casing needed
**Rationale**: V1 state trees store rollover_fee on the merkle tree (covers tree + queue rent). V2 state trees store rollover_fee only on the output queue (merkle tree has 0). Reading from the account's own metadata handles both versions automatically.
**Alternatives considered**:
- Version-specific formula (rejected: unnecessary, metadata is self-describing)

## R9: Partial reimbursement from PDA

**Decision**: All-or-nothing: 5000 or 0
**Rationale**: If PDA has less than 5000 beyond rent exemption, no reimbursement. Simplifies logic, avoids partial transfer edge cases.
**Alternatives considered**:
- `min(pda_balance, 5000)` partial transfer (rejected: adds complexity, partial reimbursement still leaves forester net-negative)

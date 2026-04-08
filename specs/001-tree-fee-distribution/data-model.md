# Data Model: Tree Fee Distribution

## New Entities

### ReimbursementPda (registry program)

Per-tree escrow account for batch_nullify reimbursements.

- **Owner**: Registry program
- **Seeds**: `[b"reimbursement", tree_pubkey.as_ref()]`
- **Fields**: None (Anchor account with discriminator only; lamport balance is the state)
- **Rent-exempt minimum**: ~890,880 lamports (8-byte discriminator account). PDA disburses only when `lamports >= rent_exempt + 5000`.
- **Relationships**: One-to-one with state tree merkle tree accounts
- **Lifecycle**: Created via `init_reimbursement_pda`, funded by batch_append registry wrapper, disbursed by batch_nullify registry wrapper

### ProtocolConfig.protocol_fee_recipient (registry program, modified)

Repurposed from existing `place_holder: Pubkey` field.

- **Type**: `Pubkey`
- **Default**: `Pubkey::default()` (zero address = no fee claiming)
- **Updated via**: `update_protocol_config` instruction (authority-gated)
- **Used by**: Registry `claim_fees` wrapper passes this as `fee_recipient` to account-compression

## Modified Entities

### RolloverMetadata (program-libs/merkle-tree-metadata)

No structural changes. Fields read by claim_fees:
- `rollover_fee: u64` -- per-leaf rollover fee
- `network_fee: u64` -- per-operation network fee (used to gate reimbursement logic)

### BatchedMerkleTreeMetadata (program-libs/batched-merkle-tree)

No structural changes. Fields read by claim_fees:
- `capacity: u64` -- total leaf capacity
- `next_index: u64` -- next leaf index (paid fees = next_index - 1)

## Constants

### Fee Cap

```
FORESTER_REIMBURSEMENT_CAP: u64 = 5000  // lamports
```

### Hardcoded Rent Exemption

Per-account-type constants derived from `solana rent <size>` at deployment:

```
// Exact values TBD at deployment -- derived from account sizes
RENT_V2_STATE_TREE: u64 = <solana rent <v2_state_tree_size>>
RENT_V2_OUTPUT_QUEUE: u64 = <solana rent <v2_output_queue_size>>
RENT_V2_ADDRESS_TREE: u64 = <solana rent <v2_address_tree_size>>
RENT_V1_STATE_TREE: u64 = <solana rent <v1_state_tree_size>>
RENT_V1_ADDRESS_TREE: u64 = <solana rent <v1_address_tree_size>>
```

Note: V1 trees have varying sizes depending on initialization parameters (height, changelog_size, roots_size). All deployed mainnet configurations must be enumerated and a rent constant hardcoded per account size. The Solana rent formula cannot be used as a pure function because the rate (currently 3480 lamports/byte/year) will change. A lookup from `account.data_len()` to the correct rent constant is the implementation approach.

## New Error Variants

### AccountCompressionErrorCode (account-compression program)

| Error | Description |
|-------|-------------|
| `InvalidAccountType` | Account passed to claim_fees is not a supported tree/queue type |
| `NoExcessFees` | Claimable excess is zero or negative (optional: may be a no-op instead of error) |

### RegistryError (registry program)

| Error | Description |
|-------|-------------|
| `InvalidFeeRecipient` | fee_recipient does not match protocol_config.protocol_fee_recipient |
| `InvalidTreeForReimbursementPda` | Tree account is not a valid state tree (wrong owner, discriminator, or is an address tree) |

## State Transitions

### Reimbursement PDA Balance

```
init_reimbursement_pda:  0 -> rent_exempt_minimum
batch_append (registry): balance += FORESTER_REIMBURSEMENT_CAP (5000, from forester, if network_fee >= 5000)
batch_nullify (registry): balance -= 5000 (to forester, if balance >= rent_exempt + 5000)
nullify_leaves (registry): balance += (network_fee - 5000) (from forester, if network_fee > 5000)
```

### Tree/Queue Account Balance (claim_fees)

```
Before: balance = rent + accumulated_rollover_fees + accumulated_network_fees
After:  balance = hardcoded_rent + rollover_fee * (capacity - next_index + 1)
Transferred: balance_before - balance_after -> fee_recipient
```

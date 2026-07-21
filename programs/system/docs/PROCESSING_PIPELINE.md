# Processing Pipeline

## Overview

The Light System Program processes compressed account state transitions through a comprehensive pipeline that validates inputs, verifies ZK proofs, and coordinates with the account-compression program via CPI.

**Source:** `programs/system/src/processor/process.rs`

## Input Types

The processing pipeline handles four types of compressed account inputs:

### 1. Writable Compressed Accounts
Input accounts that will be nullified and potentially create new outputs.

**Field:** `inputs.input_compressed_accounts_with_merkle_context`

**Operations:**
- Sum check lamports (input + compression = output + decompression)
- Compress or decompress lamports
- Hash input compressed accounts
- Insert nullifiers into queue
- Verify inclusion (by index or ZKP)

### 2. Read-Only Compressed Accounts
Accounts that are verified to exist but not modified.

**Field:** `read_only_accounts`

**Operations:**
- Verify inclusion (by index or ZKP)
- Check no duplicates with writable inputs

### 3. New Addresses
New compressed account addresses to be created.

**Field:** `inputs.new_address_params`

**Operations:**
- Derive addresses from seed and invoking program
- Insert addresses into address Merkle tree queue
- Verify non-inclusion (address doesn't exist yet)

### 4. Read-Only Addresses
Addresses verified to exist without modification.

**Field:** `read_only_addresses`

**Operations:**
- Verify non-inclusion in bloom filter queue
- Verify inclusion by ZKP

---

## Processing Steps

### Step 1: Allocate CPI Data and Initialize Context

```rust
let (mut context, mut cpi_ix_bytes) = create_cpi_data_and_context(
    ctx,
    num_output_compressed_accounts,
    num_input_accounts,
    num_new_addresses,
    hashed_pubkeys_capacity,
    cpi_outputs_data_len,
    invoking_program,
    remaining_accounts,
)?;
```

**Purpose:** Pre-allocate memory for CPI instruction data and create the processing context.

**Context includes:**
- Hashed pubkey cache for efficiency
- Fee tracking for rollover and protocol fees
- Address collection for validation

### Step 2: Deserialize and Validate Merkle Tree Accounts

```rust
let mut accounts = try_from_account_infos(remaining_accounts, &mut context)?;
```

**Purpose:** Deserialize all Merkle tree and queue accounts from remaining accounts, performing validation checks.

**Validates:**
- Account ownership (account-compression program)
- Account discriminators
- Tree heights and parameters

### Step 3: Deserialize CPI Instruction Data

```rust
let (mut cpi_ix_data, bytes) = InsertIntoQueuesInstructionDataMut::new_at(
    &mut cpi_ix_bytes[12..],
    num_output_compressed_accounts,
    num_input_accounts,
    num_new_addresses,
    min(remaining_accounts.len(), num_output_compressed_accounts),
    min(remaining_accounts.len(), num_input_accounts),
    min(remaining_accounts.len(), num_new_addresses),
)?;
```

**Purpose:** Initialize zero-copy instruction data structure for account-compression CPI.

### Step 4: Read Address Roots

```rust
let address_tree_height = read_address_roots(
    accounts.as_slice(),
    inputs.new_addresses(),
    read_only_addresses,
    &mut new_address_roots,
)?;
```

**Purpose:** Read Merkle roots from address trees for proof verification.

### Step 5: Collect Existing Addresses

```rust
inputs.input_accounts().for_each(|account| {
    context.addresses.push(account.address());
});
```

**Purpose:** Collect all addresses from input accounts to validate that output accounts only use existing or new addresses.

### Step 6: Derive New Addresses

```rust
derive_new_addresses::<ADDRESS_ASSIGNMENT>(
    inputs.new_addresses(),
    remaining_accounts,
    &mut context,
    &mut cpi_ix_data,
    accounts.as_slice(),
)?;
```

**Purpose:** Derive new addresses from seeds and invoking program, validate address assignments to output accounts.

### Step 7: Verify Read-Only Address Non-Inclusion

```rust
verify_read_only_address_queue_non_inclusion(
    accounts.as_mut_slice(),
    inputs.read_only_addresses().unwrap_or_default(),
)?;
```

**Purpose:** Verify read-only addresses aren't in pending bloom filter queues (would indicate address not yet in tree).

### Step 8: Process Outputs

```rust
let output_compressed_account_hashes = create_outputs_cpi_data::<T>(
    &inputs,
    remaining_accounts,
    &mut context,
    &mut cpi_ix_data,
    accounts.as_slice(),
)?;
```

**Purpose:** Prepare output compressed accounts:
- Compute output account hashes
- Validate output Merkle tree indices are in order
- Check new address assignments are valid
- Collect output queue/tree accounts

### Step 9: Process Inputs

```rust
let input_compressed_account_hashes = create_inputs_cpi_data(
    remaining_accounts,
    &inputs,
    &mut context,
    &mut cpi_ix_data,
    accounts.as_slice(),
)?;
```

**Purpose:** Process input compressed accounts:
- Hash input compressed accounts for nullification
- Collect input queue/tree accounts

### Step 10: Create Transaction Hash

```rust
if inputs.with_transaction_hash() {
    cpi_ix_data.tx_hash = create_tx_hash_from_hash_chains(
        &input_compressed_account_hashes,
        &output_compressed_account_hashes,
        current_slot,
    )?;
}
```

**Purpose:** Create transaction hash from input and output hash chains for transaction tracking (optional).

### Step 11: Check No Duplicate Accounts

```rust
check_no_duplicate_accounts_in_inputs_and_read_only(
    &cpi_ix_data.nullifiers,
    read_only_accounts,
)?;
```

**Purpose:** Validate no account appears in both input (writable) and read-only arrays.

### Step 12: Sum Check

```rust
let num_input_accounts_by_index = sum_check(&inputs, &None, &inputs.is_compress())?;
```

**Purpose:** Verify lamport conservation:
```
input_lamports + compress_lamports = output_lamports + decompress_lamports
```

### Step 13: SOL Compression/Decompression

```rust
compress_or_decompress_lamports::<A, T>(&inputs, ctx)?;
```

**Purpose:** Transfer SOL between:
- **Compress:** User account -> Sol Pool PDA (creates compressed SOL)
- **Decompress:** Sol Pool PDA -> Decompression Recipient (extracts compressed SOL)

### Step 14: Verify Read-Only Account Inclusion by Index

```rust
let num_read_only_accounts_by_index =
    verify_read_only_account_inclusion_by_index(accounts.as_mut_slice(), read_only_accounts)?;
```

**Purpose:** Verify read-only accounts exist by checking their inclusion in output queues using indexed lookup.

### Step 15: Read State Roots

```rust
let state_tree_height = read_input_state_roots(
    accounts.as_slice(),
    inputs.input_accounts(),
    read_only_accounts,
    &mut input_compressed_account_roots,
)?;
```

**Purpose:** Read Merkle roots from state trees for accounts that will be proven by ZKP (not by index).

### Step 16: Verify ZK Proof

```rust
verify_proof(
    &input_compressed_account_roots,
    &proof_input_compressed_account_hashes,
    &new_address_roots,
    &new_addresses,
    &compressed_proof,
    address_tree_height,
    state_tree_height,
)?;
```

**Purpose:** Verify the ZK proof covering:
1. Input compressed account inclusion (state tree)
2. Read-only account inclusion (state tree)
3. New address non-inclusion (address tree)
4. Read-only address inclusion (address tree)

**Proof inputs order:**
1. Input compressed accounts (not proven by index)
2. Read-only compressed accounts (not proven by index)
3. New addresses
4. Read-only addresses

### Step 17: Transfer Fees

```rust
context.transfer_fees(remaining_accounts, ctx.get_fee_payer())?;
```

**Purpose:** Transfer network, address, and rollover fees from fee payer to appropriate recipients.

**Note:** Rollover fees are transferred from the system program instead of the account-compression program to reduce CPI depth.

### Step 18: Copy CPI Context Outputs

```rust
copy_cpi_context_outputs(inputs.get_cpi_context_account(), bytes)?;
```

**Purpose:** If using CPI context, copy output account data to ensure all data is emitted in the transaction for indexing.

### Step 19: CPI Account Compression Program

```rust
cpi_account_compression_program(context, cpi_ix_bytes)
```

**Purpose:** Execute CPI to account-compression program to:
- Insert nullifiers into nullifier queues
- Append output states to output queues
- Insert new addresses into address queues

---

## Processing Flow Diagram

```
                          +--------------------------+
                          |   Instruction Entry      |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 1. Allocate CPI Data     |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 2. Deserialize Accounts  |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 3. Deserialize CPI Data  |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 4. Read Address Roots    |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 5. Collect Addresses     |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 6. Derive New Addresses  |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 7. Verify Read-Only      |
                          |    Address Non-Inclusion |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 8. Process Outputs       |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 9. Process Inputs        |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 10. Create Tx Hash       |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 11. Check No Duplicates  |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 12. Sum Check            |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 13. SOL Compress/        |
                          |     Decompress           |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 14. Verify Read-Only     |
                          |     by Index             |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 15. Read State Roots     |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 16. Verify ZK Proof      |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 17. Transfer Fees        |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 18. Copy CPI Context     |
                          +--------------------------+
                                      |
                                      v
                          +--------------------------+
                          | 19. CPI Account          |
                          |     Compression          |
                          +--------------------------+
```

---

## Proof Verification Modes

### 1. Proof by Index
For recently inserted accounts, inclusion can be verified by checking the account exists at a specific index in the output queue.

```rust
if input_account.prove_by_index() {
    // Verify by checking output queue at leaf_index
}
```

### 2. Proof by ZKP
For accounts in the Merkle tree (not in queue), inclusion is verified via ZK proof.

```rust
if !input_account.prove_by_index() {
    // Include in ZK proof verification
}
```

---

## Related Files

| File | Purpose |
|------|---------|
| `src/processor/process.rs` | Main processing pipeline |
| `src/processor/cpi.rs` | CPI to account-compression program |
| `src/processor/create_address_cpi_data.rs` | Address derivation and CPI data |
| `src/processor/create_inputs_cpi_data.rs` | Input account processing |
| `src/processor/create_outputs_cpi_data.rs` | Output account processing |
| `src/processor/read_only_account.rs` | Read-only account verification |
| `src/processor/read_only_address.rs` | Read-only address verification |
| `src/processor/sol_compression.rs` | SOL compression/decompression |
| `src/processor/sum_check.rs` | Lamport conservation check |
| `src/processor/verify_proof.rs` | ZK proof verification |

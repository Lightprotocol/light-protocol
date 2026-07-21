# CPI Context

## Quick Reference

```rust
// 1. Initialize CPI context account (once)
initialize_cpi_context_account(state_tree_pubkey, capacity_params);

// 2. First program invocation - clears and writes
CompressedCpiContext::first()  // { first_set_context: true, set_context: false }

// 3. Additional program invocations - append (0 or more)
CompressedCpiContext::set()    // { first_set_context: false, set_context: true }

// 4. Final program invocation - execute
CompressedCpiContext::default() // { first_set_context: false, set_context: false }
```

**Critical rules:**
- Same fee payer across all invocations
- Same state Merkle tree across all invocations
- One proof in final invocation covers all accounts
- CPI context automatically cleared after execution

---

## Overview

CPI Context is a coordination mechanism that enables multiple Solana programs to share a single zero-knowledge proof when operating on compressed accounts. Without CPI Context, each program would need its own proof, multiplying compute costs and transaction size.

**Key Benefits:**
- Share one ZK proof across multiple programs (50-75% cost reduction)
- Enable complex multi-program workflows (DeFi compositions, token + PDA updates)
- Maintain security - each program validates its own accounts before contributing to shared proof

**How it works:**
1. First program writes its account data to CPI context account
2. Additional programs append their account data
3. Final program reads all data, verifies one proof covering everything, executes transaction

**Source:** `programs/system/src/cpi_context/`

---

## State Machine

```
                 +-------------------+
                 |   Empty/Cleared   |
                 |   (after init or  |
                 |    after execute) |
                 +-------------------+
                          |
                          | first_set_context = true
                          | (clear & set fee payer)
                          v
                 +-------------------+
                 |  First Set        |
                 |  - fee payer set  |
                 |  - data stored    |
                 +-------------------+
                          |
                          | set_context = true (0 or more times)
                          | (validate fee payer & append data)
                          v
                 +-------------------+
                 |  Accumulating     |
                 |  - multiple       |
                 |    programs' data |
                 +-------------------+
                          |
                          | both flags = false
                          | (read all data & execute)
                          v
                 +-------------------+
                 |  Executed         |
                 |  - proof verified |
                 |  - state updated  |
                 |  - account cleared|
                 +-------------------+
                          |
                          v
                 (back to Empty/Cleared)
```

### Modes

| Mode | Flags | Behavior |
|------|-------|----------|
| **First Set** | `first_set_context = true` | Clear account, set fee payer, store instruction data, return early |
| **Set Context** | `set_context = true` | Validate fee payer, append data, return early |
| **Execute** | Both `false` | Read all data, verify proof, execute state transition, clear account |

---

## CPI Context Flow

### Step 1: First Invocation

```rust
cpi_context: Some(CompressedCpiContext {
    first_set_context: true,
    set_context: false,
    cpi_context_account_index: 0,
})
```

**Processing:**
1. Clear entire account (zero out all data)
2. Set `fee_payer` field
3. Store instruction data (inputs, outputs, addresses)
4. Return early (no proof verification)

### Step 2: Subsequent Invocations (0 or more)

```rust
cpi_context: Some(CompressedCpiContext {
    first_set_context: false,
    set_context: true,
    cpi_context_account_index: 0,
})
```

**Processing:**
1. Validate fee payer matches first invocation
2. Validate account is not empty
3. Append instruction data to existing vectors
4. Return early (no proof verification)

### Step 3: Final Invocation

```rust
cpi_context: Some(CompressedCpiContext {
    first_set_context: false,
    set_context: false,
    cpi_context_account_index: 0,
})
```

**Processing:**
1. Validate fee payer matches
2. Read all accumulated data from CPI context
3. Combine with current instruction data
4. Verify single ZK proof against all accounts
5. Execute complete state transition
6. Clear CPI context account

---

## Data Structures

### CompressedCpiContext (Instruction Data)

```rust
pub struct CompressedCpiContext {
    pub first_set_context: bool,        // Clear and write
    pub set_context: bool,              // Append
    pub cpi_context_account_index: u8,  // Index in remaining accounts
}
```

**Source:** `program-libs/compressed-account/src/instruction_data/cpi_context.rs`

### Stored Data in CpiContextAccount

| Field | Purpose |
|-------|---------|
| `fee_payer` | Transaction fee payer (set on first_set_context) |
| `associated_merkle_tree` | Required Merkle tree association |
| `new_addresses` | Addresses to create |
| `readonly_addresses` | Read-only address references |
| `readonly_accounts` | Read-only account references |
| `in_accounts` | Input compressed accounts |
| `out_accounts` | Output compressed accounts |
| `output_data` | Variable-length output account data |

---

## Validation Rules

### 1. Fee Payer Consistency
All invocations must use the same fee payer. Set during `first_set_context`, validated on all subsequent operations.

```rust
if *cpi_context_account.fee_payer != fee_payer {
    return Err(SystemProgramError::CpiContextFeePayerMismatch);
}
```

### 2. Merkle Tree Association
CPI context must be associated with the first Merkle tree used in the transaction.

```rust
if *cpi_context_account.associated_merkle_tree != first_merkle_tree_pubkey {
    return Err(SystemProgramError::CpiContextAssociatedMerkleTreeMismatch);
}
```

### 3. Non-Empty on Execute
Account must contain data when executing.

```rust
if cpi_context_account.is_empty() {
    return Err(SystemProgramError::CpiContextEmpty);
}
```

### 4. Account Must Exist for Context Operations
Cannot use `set_context` or `first_set_context` flags without passing the CPI context account.

---

## Limitations

### 1. Read-Only Accounts/Addresses
Not supported when writing to CPI context account. Include read-only accounts only in the final execute invocation.

### 2. Fixed Capacity
CPI context accounts have fixed capacity determined at initialization:

```rust
pub struct CpiContextAccountInitParams {
    pub new_addresses_len: u8,        // Default: 10
    pub readonly_addresses_len: u8,   // Default: 10
    pub readonly_accounts_len: u8,    // Default: 10
    pub in_accounts_len: u8,          // Default: 20
    pub out_accounts_len: u8,         // Default: 30
}
```

**Default account size:** 14,020 bytes

Overflow returns `ZeroCopyError::InsufficientCapacity` and transaction fails.

### 3. Single Merkle Tree Association
All operations in a CPI context transaction must use the same state Merkle tree.

---

## Error Codes

| Code | Error | Cause |
|------|-------|-------|
| 6020 | CpiContextAccountUndefined | CPI context account required but not provided |
| 6021 | CpiContextEmpty | Account empty during execute mode |
| 6022 | CpiContextMissing | CPI context data missing in instruction |
| 6027 | CpiContextFeePayerMismatch | Fee payer doesn't match first invocation |
| 6028 | CpiContextAssociatedMerkleTreeMismatch | Wrong Merkle tree association |
| 6049 | CpiContextAlreadySet | Attempting to set when already set |
| 6054 | CpiContextPassedAsSetContext | Account doesn't exist but passed as set_context |
| 6055 | InvalidCpiContextOwner | Wrong account owner |
| 6056 | InvalidCpiContextDiscriminator | Wrong discriminator |
| 6064 | CpiContextDeactivated | CPI context is deactivated |

---

## Related Files

| File | Purpose |
|------|---------|
| `src/cpi_context/state.rs` | CpiContextAccount structure and serialization |
| `src/cpi_context/process_cpi_context.rs` | CPI context processing logic |
| `src/cpi_context/account.rs` | CpiContextInAccount, CpiContextOutAccount types |
| `src/cpi_context/address.rs` | CpiContextNewAddressParamsAssignedPacked type |

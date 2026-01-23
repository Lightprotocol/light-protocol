# Account Layouts

## Overview

The Light System Program owns one primary account type: **CpiContextAccount**. This account is used to collect instruction data across multiple CPI invocations before executing a combined state transition with a single ZK proof.

## 1. CpiContextAccount (Version 2)

### Description
The CpiContextAccount collects instruction data without executing a compressed transaction. It enables multi-program transactions by:
1. Caching validated instruction data from multiple CPI invocations
2. Combining all cached data with the final executing CPI
3. Executing the combined state transition with a single ZK proof

### Discriminator
```rust
CPI_CONTEXT_ACCOUNT_2_DISCRIMINATOR: [u8; 8] = [34, 184, 183, 14, 100, 80, 183, 124]
```

### Ownership
- **Owner:** Light System Program (`SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7`)

### State Layout

**Source:** `programs/system/src/cpi_context/state.rs`

```rust
pub struct ZCpiContextAccount2<'a> {
    pub fee_payer: Ref<&'a mut [u8], Pubkey>,              // 32 bytes - Transaction fee payer
    pub associated_merkle_tree: Ref<&'a mut [u8], Pubkey>, // 32 bytes - Associated state Merkle tree
    _associated_queue: Ref<&'a mut [u8], Pubkey>,          // 32 bytes - Placeholder for future queue
    _place_holder_bytes: Ref<&'a mut [u8], [u8; 32]>,     // 32 bytes - Reserved
    pub new_addresses: ZeroCopyVecU8<'a, CpiContextNewAddressParamsAssignedPacked>, // Variable
    pub readonly_addresses: ZeroCopyVecU8<'a, ZPackedReadOnlyAddress>,              // Variable
    pub readonly_accounts: ZeroCopyVecU8<'a, ZPackedReadOnlyCompressedAccount>,     // Variable
    pub in_accounts: ZeroCopyVecU8<'a, CpiContextInAccount>,                        // Variable
    pub out_accounts: ZeroCopyVecU8<'a, CpiContextOutAccount>,                      // Variable
    total_output_data_len: Ref<&'a mut [u8], U16>,         // 2 bytes - Total serialized output size
    output_data_len: Ref<&'a mut [u8], U16>,               // 2 bytes - Number of output data entries
    pub output_data: Vec<ZeroCopySliceMut<'a, U16, u8>>,  // Variable - Output account data
    remaining_data: &'a mut [u8],                          // Remaining capacity
}
```

**Fixed Header Size:** 8 (discriminator) + 32 + 32 + 32 + 32 = 136 bytes

**Note:** The `Ref<&'a mut [u8], T>` wrapper is used for zero-copy access to fixed-size fields. The lifetime parameter `'a` ensures the account data remains borrowed for the duration of the operation.

### Initialization Parameters

**Source:** `programs/system/src/cpi_context/state.rs`

```rust
pub struct CpiContextAccountInitParams {
    pub associated_merkle_tree: Pubkey,
    pub associated_queue: Pubkey,           // Currently placeholder (always Pubkey::default())
    pub new_addresses_len: u8,              // Default: 10 - Pre-allocated capacity
    pub readonly_addresses_len: u8,         // Default: 10 - Pre-allocated capacity
    pub readonly_accounts_len: u8,          // Default: 10 - Pre-allocated capacity
    pub in_accounts_len: u8,                // Default: 20 - Pre-allocated capacity
    pub out_accounts_len: u8,               // Default: 30 - Pre-allocated capacity
}
```

**Capacity Planning:**
The length parameters specify pre-allocated capacity for each vector collection. This capacity determines the maximum number of items that can be stored without resizing. Choose values based on expected transaction complexity:
- Simple transactions: Use defaults (10/10/10/20/30)
- Complex multi-program transactions: Increase capacity as needed
- Each item has a fixed size (see Supporting Types section for exact sizes)

**Default Account Size:** `DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE_V2` = 14020 bytes

**Source:** `program-libs/batched-merkle-tree/src/constants.rs`

**Size Calculation Example with Defaults:**
```
Fixed header: 136 bytes
Vector metadata (5 vectors × 2 bytes): 10 bytes
New addresses (10 × 70 bytes): 700 bytes
Readonly addresses (10 × 36 bytes): 360 bytes
Readonly accounts (10 × 48 bytes): 480 bytes
Input accounts (20 × 144 bytes): 2880 bytes
Output accounts (30 × 128 bytes): 3840 bytes
Output data metadata: 4 bytes
Remaining buffer for output data: ~5610 bytes
Total: 14020 bytes
```

**Note:** The remaining buffer is used for variable-length output data storage (compressed account data payloads).

### Serialization

Uses zero-copy serialization via `light_zero_copy` crate for performance:
- **`Ref<&'a mut [u8], T>`** - Zero-copy wrapper for fixed-size fields (fee_payer, associated_merkle_tree, etc.)
- **`ZeroCopyVecU8<'a, T>`** - Variable-length vectors with u8 length prefix (2 bytes: 1 byte length + 1 byte capacity)
- **`ZeroCopySliceMut<'a, U16, u8>`** - Output data slices with u16 length prefix

**Memory Layout:**
```
[8 bytes discriminator]
[32 bytes fee_payer]
[32 bytes associated_merkle_tree]
[32 bytes _associated_queue]
[32 bytes _place_holder_bytes]
[2 bytes new_addresses metadata][variable new_addresses data]
[2 bytes readonly_addresses metadata][variable readonly_addresses data]
[2 bytes readonly_accounts metadata][variable readonly_accounts data]
[2 bytes in_accounts metadata][variable in_accounts data]
[2 bytes out_accounts metadata][variable out_accounts data]
[2 bytes total_output_data_len]
[2 bytes output_data_len]
[variable output_data with u16 length prefixes]
[remaining unallocated space]
```

### Associated Instructions

| Instruction | Purpose | Context Flag | Effect |
|-------------|---------|--------------|--------|
| `InitializeCpiContextAccount` | Create new V2 account | N/A | Initializes discriminator, sets tree, pre-allocates capacity |
| `ReInitCpiContextAccount` | Migrate V1 to V2 | N/A | Validates V1, zeros data, writes V2 discriminator, resizes |
| `InvokeCpi` | Standard CPI (Anchor) | `set_context=true` | Writes/appends instruction data to context |
| `InvokeCpiWithReadOnly` | CPI with read-only | `set_context=true` | Writes/appends with read-only account support |
| `InvokeCpiWithAccountInfo` | CPI V2 mode | `first_set_context=true` | First write to context (validates empty) |
| `InvokeCpiWithAccountInfo` | CPI V2 mode | `set_context=true` | Append to existing context data |
| `InvokeCpi*` (any variant) | Execute transaction | `set_context=false` | Reads all context data, executes, auto-clears |

**Documentation Links:**
- [INIT_CPI_CONTEXT_ACCOUNT.md](init/INIT_CPI_CONTEXT_ACCOUNT.md) - Create new CPI context account
- [REINIT_CPI_CONTEXT_ACCOUNT.md](init/REINIT_CPI_CONTEXT_ACCOUNT.md) - Migrate from V1 to V2
- [INVOKE_CPI.md](invoke_cpi/INVOKE_CPI.md) - Standard CPI invocation (Anchor mode)
- [INVOKE_CPI_WITH_READ_ONLY.md](invoke_cpi/INVOKE_CPI_WITH_READ_ONLY.md) - CPI with read-only support
- [INVOKE_CPI_WITH_ACCOUNT_INFO.md](invoke_cpi/INVOKE_CPI_WITH_ACCOUNT_INFO.md) - CPI V2 mode (dynamic accounts)

**Context Flag Behavior:**
- `first_set_context=true`: Validates account is empty, then writes data (prevents overwriting existing context)
- `set_context=true`: Appends data to existing context (allows multi-program collection)
- Both flags false: Executes transaction using all collected data, then clears account

---

## 2. CpiContextAccount (Version 1 - Legacy)

### Description
Legacy version of the CPI context account. Should be migrated to version 2 using `ReInitCpiContextAccount`.

**Key Differences from V2:**
- Uses Borsh serialization instead of zero-copy
- Only stores fee_payer and associated_merkle_tree
- No support for collecting instruction data across multiple CPIs
- Smaller size (minimal state storage)

### Discriminator
```rust
CPI_CONTEXT_ACCOUNT_1_DISCRIMINATOR: [u8; 8] = [22, 20, 149, 218, 74, 204, 128, 166]
```

**Source:** `programs/system/src/constants.rs`

### State Layout (Borsh Serialization)

**Source:** `programs/system/src/cpi_context/state.rs`

```rust
#[derive(BorshDeserialize)]
pub struct CpiContextAccount {
    pub fee_payer: Pubkey,              // 32 bytes - Transaction fee payer
    pub associated_merkle_tree: Pubkey, // 32 bytes - Associated state Merkle tree
}
```

**Total Size:** 8 (discriminator) + 32 + 32 = 72 bytes (minimum account size)

### Migration
Use `ReInitCpiContextAccount` instruction to migrate from version 1 to version 2. The migration process:
1. Validates V1 discriminator
2. Zeros out account data
3. Writes V2 discriminator
4. Initializes V2 structure with default capacity parameters
5. Resizes account to 14020 bytes

See [REINIT_CPI_CONTEXT_ACCOUNT.md](init/REINIT_CPI_CONTEXT_ACCOUNT.md) for details.

---

## Version Comparison (V1 vs V2)

| Feature | Version 1 (Legacy) | Version 2 (Current) |
|---------|-------------------|---------------------|
| **Discriminator** | `[22, 20, 149, 218, 74, 204, 128, 166]` | `[34, 184, 183, 14, 100, 80, 183, 124]` |
| **Serialization** | Borsh | Zero-copy (light_zero_copy) |
| **Default Size** | 72 bytes (minimal) | 14020 bytes (configurable) |
| **State Fields** | fee_payer, associated_merkle_tree | fee_payer, associated_merkle_tree, plus 5 vector collections |
| **CPI Support** | Basic single-program | Multi-program with context collection |
| **New Addresses** | Not supported | Up to 10 (default) |
| **Input Accounts** | Not stored | Up to 20 (default) |
| **Output Accounts** | Not stored | Up to 30 (default) |
| **Read-Only Support** | No | Yes (addresses and accounts) |
| **Output Data Storage** | No | Variable-length with u16 prefixes |
| **Performance** | Lower (Borsh deserialization) | Higher (zero-copy access) |
| **Use Case** | Single program invocation | Multi-program transactions with shared ZK proof |

**Migration Path:** V1 → V2 via `ReInitCpiContextAccount` instruction

---

## 3. Supporting Types

All supporting types use the zerocopy crate's derive macros for safe zero-copy serialization:
- **`FromBytes`** - Safe deserialization from byte slices
- **`IntoBytes`** - Safe serialization to byte slices
- **`KnownLayout`** - Compile-time memory layout verification
- **`Immutable`** - Guarantees no interior mutability
- **`Unaligned`** - No alignment requirements (packed layout)

These traits ensure memory safety and prevent undefined behavior when casting between byte slices and struct references.

### CpiContextInAccount

Represents an input compressed account stored in CPI context.

**Source:** `programs/system/src/cpi_context/account.rs`

```rust
#[repr(C)]
pub struct CpiContextInAccount {
    pub owner: Pubkey,                        // 32 bytes - Account owner
    pub has_data: u8,                         // 1 byte - Has compressed data flag
    pub discriminator: [u8; 8],               // 8 bytes - Data discriminator
    pub data_hash: [u8; 32],                  // 32 bytes - Hash of account data
    pub merkle_context: ZPackedMerkleContext, // 12 bytes - Merkle tree context (with padding)
    pub root_index: U16,                      // 2 bytes - Merkle root index
    pub lamports: U64,                        // 8 bytes - Account lamports
    pub with_address: u8,                     // 1 byte - Has address flag
    pub address: [u8; 32],                    // 32 bytes - Optional address
}
```

**Size:** 144 bytes (includes alignment padding)

**Note:** The actual size is 144 bytes due to Rust struct alignment and padding, not 124 bytes as simple field addition would suggest.

**Trait Implementations:**
- `InputAccount<'_>` - Provides methods to access input account fields (owner, lamports, address, merkle_context, has_data, data, hash_with_hashed_values, root_index)
- Used during CPI context processing to validate and hash input accounts

### CpiContextOutAccount

Represents an output compressed account stored in CPI context.

**Source:** `programs/system/src/cpi_context/account.rs`

```rust
#[repr(C)]
pub struct CpiContextOutAccount {
    pub owner: Pubkey,                    // 32 bytes - Account owner
    pub has_data: u8,                     // 1 byte - Has compressed data flag
    pub discriminator: [u8; 8],           // 8 bytes - Data discriminator
    pub data_hash: [u8; 32],              // 32 bytes - Hash of account data
    pub output_merkle_tree_index: u8,     // 1 byte - Output tree index
    pub lamports: U64,                    // 8 bytes - Account lamports
    pub with_address: u8,                 // 1 byte - Has address flag
    pub address: [u8; 32],                // 32 bytes - Optional address
}
```

**Size:** 128 bytes (includes alignment padding)

**Note:** The actual size is 128 bytes due to Rust struct alignment and padding, not 115 bytes as simple field addition would suggest.

**Trait Implementations:**
- `OutputAccount<'_>` - Provides methods to access output account fields (owner, lamports, address, has_data, data, merkle_tree_index, hash_with_hashed_values)
- Used during CPI context processing to validate and hash output accounts before inserting into Merkle trees

### CpiContextNewAddressParamsAssignedPacked

Represents a new address to be created, stored in CPI context.

**Source:** `programs/system/src/cpi_context/address.rs`

```rust
#[repr(C)]
pub struct CpiContextNewAddressParamsAssignedPacked {
    pub owner: [u8; 32],                       // 32 bytes - Address owner (program)
    pub seed: [u8; 32],                        // 32 bytes - Address seed
    pub address_queue_account_index: u8,       // 1 byte - Queue account index
    pub address_merkle_tree_account_index: u8, // 1 byte - Tree account index
    pub address_merkle_tree_root_index: U16,   // 2 bytes - Root index
    pub assigned_to_account: u8,               // 1 byte - Is assigned flag
    pub assigned_account_index: u8,            // 1 byte - Assigned output index
}
```

**Size:** 70 bytes

**Trait Implementations:**
- `NewAddress<'_>` - Provides methods to access new address parameters (seed, address_queue_index, address_merkle_tree_account_index, address_merkle_tree_root_index, assigned_compressed_account_index, owner)
- Used during address derivation and validation in the processing pipeline

### ZPackedReadOnlyAddress

Represents a read-only address to be verified (exists in address Merkle tree).

**Source:** `program-libs/compressed-account/src/instruction_data/zero_copy.rs`

```rust
#[repr(C)]
pub struct ZPackedReadOnlyAddress {
    pub address: [u8; 32],                         // 32 bytes - Address to verify
    pub address_merkle_tree_root_index: U16,       // 2 bytes - Root index
    pub address_merkle_tree_account_index: u8,     // 1 byte - Tree account index
}
```

**Size:** 36 bytes (includes 1 byte alignment padding)

### ZPackedReadOnlyCompressedAccount

Represents a read-only compressed account to be verified (exists in state Merkle tree).

**Source:** `program-libs/compressed-account/src/instruction_data/zero_copy.rs`

```rust
#[repr(C)]
pub struct ZPackedReadOnlyCompressedAccount {
    pub account_hash: [u8; 32],                 // 32 bytes - Account hash
    pub merkle_context: ZPackedMerkleContext,   // 12 bytes - Merkle tree context
    pub root_index: U16,                        // 2 bytes - Root index
}
```

**Size:** 48 bytes (includes 2 bytes alignment padding)

### ZPackedMerkleContext

Packed Merkle tree context for input accounts.

**Source:** `program-libs/compressed-account/src/instruction_data/zero_copy.rs`

```rust
#[repr(C)]
pub struct ZPackedMerkleContext {
    pub merkle_tree_pubkey_index: u8,  // 1 byte - Index in remaining accounts
    pub queue_pubkey_index: u8,        // 1 byte - Queue index in remaining accounts
    pub leaf_index: U32,               // 4 bytes - Leaf index in tree
    pub prove_by_index: u8,            // 1 byte - Prove by index flag (0 or 1)
}
```

**Size:** 12 bytes (includes 5 bytes of alignment padding after prove_by_index)

**Note:** Due to `U32` alignment requirements, the compiler adds padding to align the struct to 4-byte boundaries, resulting in 12 bytes total instead of 7 bytes.

---

## 4. Account Lifecycle

### Creation
1. User creates account via System Program with rent exemption
2. Calls `InitializeCpiContextAccount` instruction
3. Account is resized to `DEFAULT_CPI_CONTEXT_ACCOUNT_SIZE_V2` (14020 bytes)
4. Discriminator is set to `CPI_CONTEXT_ACCOUNT_2_DISCRIMINATOR`
5. Fee payer is initialized to zero (set during first use)
6. Associated Merkle tree is set to specified tree pubkey
7. Vector capacities are pre-allocated based on initialization parameters

### Usage Pattern

**Single-Program Transaction (Direct Invoke):**
```
User → Invoke instruction → Process → Account Compression CPI
```
- No CPI context account needed
- All data passed in instruction data

**Multi-Program Transaction (CPI Context):**
```
Program A → InvokeCpi (set_context=true) → Write to CPI context
Program B → InvokeCpi (set_context=true) → Append to CPI context
Program C → InvokeCpi (set_context=false) → Execute with combined data
```
1. **First CPI (set_context=true):** Writes instruction data to CPI context account
2. **Additional CPIs (set_context=true):** Append more instruction data
3. **Final CPI (set_context=false):** Combines all data, executes with single ZK proof
4. **Auto-clear:** Account is cleared after successful execution

### State Transitions

```
[Empty] → [Collecting] → [Executing] → [Empty]
   ↑         ↓   ↑           ↓           ↑
   |         |   |           |           |
   |    set_context    set_context  execute
   |      (first)      (additional)  (final)
   |                                      |
   └──────────────── clear ──────────────┘
```

**State Details:**
- **Empty:** `is_empty() == true`, ready for new transaction
- **Collecting:** Contains partial instruction data from one or more CPIs
- **Executing:** Final CPI reads all data, executes transaction, clears account
- **Error State:** If execution fails, account may need manual clearing or reinit

### Clearing

The CPI context account is automatically cleared after successful execution:
- All vector lengths set to 0
- Fee payer reset to zero
- Output data cleared
- Remaining capacity restored

**Manual Clearing:** If needed, call `deserialize_cpi_context_account_cleared()` which zeros out all data fields.

---

## Account Validation

### Ownership Check
All CPI context account operations validate ownership:
```rust
check_owner(&ID, account_info).map_err(|_| SystemProgramError::InvalidCpiContextOwner)?;
```

### Discriminator Check
Version 2 accounts must have the correct discriminator:
```rust
if discriminator != CPI_CONTEXT_ACCOUNT_2_DISCRIMINATOR {
    return Err(SystemProgramError::InvalidCpiContextDiscriminator.into());
}
```

### Association Check
CPI context accounts must be associated with the first Merkle tree used in the transaction:
```rust
if *cpi_context_account.associated_merkle_tree != first_merkle_tree_pubkey {
    return Err(SystemProgramError::CpiContextAssociatedMerkleTreeMismatch.into());
}
```

---

## Practical Examples

### Example 1: Single Program Transaction (No CPI Context)

```rust
// Direct invocation - no CPI context account needed
process_instruction(
    &system_program_id,
    accounts,
    &invoke_instruction_data,
)
```

**Use Case:** Simple compressed account transfers within one program.

### Example 2: Multi-Program Transaction

```rust
// Step 1: Program A sets initial context
invoke_cpi_instruction_data.cpi_context = Some(CpiContext {
    set_context: true,
    first_set_context: true, // Validates empty
});
program_a_invoke_cpi(&cpi_context_account, instruction_data_a)?;

// Step 2: Program B appends to context
invoke_cpi_instruction_data.cpi_context = Some(CpiContext {
    set_context: true,
    first_set_context: false, // Appends
});
program_b_invoke_cpi(&cpi_context_account, instruction_data_b)?;

// Step 3: Program C executes with combined data
invoke_cpi_instruction_data.cpi_context = Some(CpiContext {
    set_context: false, // Execute
    first_set_context: false,
});
program_c_invoke_cpi(&cpi_context_account, instruction_data_c)?;
// Account is automatically cleared after successful execution
```

**Use Case:** Complex DeFi operations requiring coordination between multiple programs (e.g., token swap + liquidity provision).

### Example 3: V1 to V2 Migration

```rust
// Check if account is V1
let discriminator = &account_data[0..8];
if discriminator == CPI_CONTEXT_ACCOUNT_1_DISCRIMINATOR {
    // Migrate to V2
    reinit_cpi_context_account(&[
        cpi_context_account,
        associated_merkle_tree_account,
        payer,
        system_program,
    ])?;
}
```

**Use Case:** Upgrading existing V1 CPI context accounts to take advantage of V2 features.

---

## Related Documentation

- **[PROCESSING_PIPELINE.md](PROCESSING_PIPELINE.md)** - How CPI context data flows through the 19-step processing pipeline
- **[CPI_CONTEXT.md](CPI_CONTEXT.md)** - Detailed explanation of multi-program transaction coordination
- **[INSTRUCTIONS.md](INSTRUCTIONS.md)** - Full instruction reference and error codes
- **[../CLAUDE.md](../CLAUDE.md)** - System program overview and source code structure

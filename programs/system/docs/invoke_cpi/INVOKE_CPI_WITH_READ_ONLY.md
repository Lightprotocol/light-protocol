# InvokeCpiWithReadOnly

## Summary

| Field | Value |
|-------|-------|
| **Discriminator** | `[86, 47, 163, 166, 21, 223, 92, 8]` |
| **Enum** | `InstructionDiscriminator::InvokeCpiWithReadOnly` |
| **Path** | `programs/system/src/lib.rs` |

## Description

Extended CPI invocation instruction that supports read-only compressed accounts and addresses. This enables programs to verify the existence of compressed state without modifying it, useful for authorization checks and multi-account validations.

### Use Cases
- Verify compressed account exists before performing operations
- Check compressed PDA state for authorization
- Read-only access to compressed token balances
- Proof-of-ownership checks without state modification

### Key Differences from InvokeCpi
| Feature | InvokeCpi | InvokeCpiWithReadOnly |
|---------|-----------|------------------------|
| **Read-only accounts** | Not supported | Supported |
| **Read-only addresses** | Not supported | Supported (execute mode only) |
| **Account mode** | Fixed (Anchor) | Configurable (Anchor/V2) |
| **Invoking program ID** | Implicit (from accounts) | Explicit (in instruction data) |

### State Changes
- Input compressed accounts are nullified (writable inputs only)
- Output compressed accounts are created
- Read-only accounts are verified but not modified
- Read-only addresses are verified to exist

---

## Instruction Data

**Source:** `program-libs/compressed-account/src/instruction_data/with_readonly.rs`

```rust
pub struct ZInstructionDataInvokeCpiWithReadOnly<'a> {
    // Metadata (fixed layout)
    pub mode: u8,  // AccountMode: 0 = Anchor, 1 = V2
    pub bump: u8,
    pub invoking_program_id: Pubkey,
    pub compress_or_decompress_lamports: U64,
    pub is_compress: bool,
    pub with_cpi_context: bool,
    pub with_transaction_hash: bool,
    pub cpi_context: ZCompressedCpiContext,

    // Variable-length fields (zero-copy slices)
    pub proof: Option<Ref<'a, [u8], CompressedProof>>,
    pub new_address_params: ZeroCopySliceBorsh<'a, ZNewAddressParamsAssignedPacked>,
    pub input_compressed_accounts: Vec<ZInAccount<'a>>,
    pub output_compressed_accounts: Vec<ZOutputCompressedAccountWithPackedContext<'a>>,
    pub read_only_addresses: ZeroCopySliceBorsh<'a, ZPackedReadOnlyAddress>,
    pub read_only_accounts: ZeroCopySliceBorsh<'a, ZPackedReadOnlyCompressedAccount>,
}
```

### Key Fields

**Mode Selection:**
- `mode`: Account mode (0 = Anchor, 1 = V2)
- `invoking_program_id`: Program ID making the CPI (embedded in instruction data)
- `bump`: PDA bump seed for signer verification

**Read-Only Features:**
- `read_only_accounts`: Compressed accounts to verify existence without modification
- `read_only_addresses`: Addresses to verify existence in address tree

---

## Accounts

Account layout depends on the `mode` field:

### Anchor Mode (mode = 0)

Same 11 accounts as [INVOKE_CPI.md](INVOKE_CPI.md):

| Index | Name | Signer | Writable | Description |
|-------|------|--------|----------|-------------|
| 0-10 | (same as InvokeCpi) | - | - | See InvokeCpi documentation |
| 11+ | `remaining_accounts` | - | - | Merkle trees, queues |

### V2 Mode (mode = 1)

Dynamic account list based on `account_option_config`:

| Index | Name | Signer | Writable | Description |
|-------|------|--------|----------|-------------|
| 0 | `fee_payer` | Yes | Yes | Pays transaction fees |
| 1 | `authority` | Yes | No | Authority (signer in V2 mode) |
| 2+ | Dynamic accounts | - | - | Determined by account_option_config |

---

## Instruction Logic

### Processing Flow
1. **CPI signer checks:** Verify invoking program authority
2. **CPI context processing:** Handle first_set_context, set_context, or execute modes
3. **Main processing pipeline:** Execute 19-step pipeline
   - Step 8: **Read-only account verification**
     - By index: Direct leaf lookup (if prove_by_index = true)
     - By ZKP: Inclusion proof verification (if prove_by_index = false)
     - Duplicate check: Ensure read-only accounts don't overlap with writable inputs
   - Step 9: **ZK proof verification** (includes read-only proofs)
4. **Read-only address verification** (execute mode only):
   - Non-inclusion check: Verify address not in bloom filter queue
   - Inclusion proof: Verify address exists in address Merkle tree

---

## Read-Only Account Processing

### Read-Only Compressed Accounts

```rust
pub struct ZPackedReadOnlyCompressedAccount {
    pub account_hash: [u8; 32],
    pub merkle_context: ZPackedMerkleContext,
    pub root_index: U16,
}
```

- `account_hash`: Pre-computed hash of the compressed account
- `merkle_context`: Merkle tree location (tree index, queue index, leaf index)
- `root_index`: Index of the Merkle root to verify against

**Verification:** Account hash inclusion in Merkle tree (by index or ZKP). Account is NOT nullified.

### Read-Only Addresses

```rust
pub struct ZPackedReadOnlyAddress {
    pub address: [u8; 32],
    pub address_merkle_tree_account_index: u8,
    pub address_merkle_tree_root_index: U16,
}
```

**Verification:**
1. Bloom filter check: Verify address is NOT in queue (not pending insertion)
2. Inclusion proof: Verify address exists in address Merkle tree

---

## Limitations

### Read-Only Addresses with CPI Context

Read-only addresses cannot be used when writing to CPI context account (first_set_context or set_context modes).

```rust
if let Some(readonly_addresses) = instruction_data.read_only_addresses() {
    if !readonly_addresses.is_empty() {
        return Err(SystemProgramError::Unimplemented)?;
    }
}
```

**Workaround:** Perform read-only address verification in the final executing program.

---

## Errors

| Code | Error | Cause |
|------|-------|-------|
| 6034 | `ReadOnlyAddressAlreadyExists` | Read-only address found in bloom filter queue |
| 6035 | `ReadOnlyAccountDoesNotExist` | Read-only account hash not found in Merkle tree |
| 6053 | `DuplicateAccountInInputsAndReadOnly` | Same account in both writable inputs and read-only |
| 6063 | `Unimplemented` | Read-only addresses with CPI context write mode |

See [INSTRUCTIONS.md](../INSTRUCTIONS.md) for complete error list.

---

## Account Mode Comparison

| Feature | Anchor Mode (0) | V2 Mode (1) |
|---------|-----------------|-------------|
| **Account count** | Fixed 11 base accounts | Dynamic (2+ base) |
| **Account order** | Strict, predefined | Flexible, config-driven |
| **Overhead** | Higher (all 11 accounts required) | Lower (only needed accounts) |
| **Best for** | Existing Anchor integrations | New programs, optimized size |

---

## Related Documentation

- [INVOKE_CPI.md](INVOKE_CPI.md) - Base CPI invocation (no read-only support)
- [INVOKE_CPI_WITH_ACCOUNT_INFO.md](INVOKE_CPI_WITH_ACCOUNT_INFO.md) - V2 mode with AccountOptions
- [PROCESSING_PIPELINE.md](../PROCESSING_PIPELINE.md) - Complete 19-step processing pipeline
- [CPI_CONTEXT.md](../CPI_CONTEXT.md) - Multi-program transaction coordination

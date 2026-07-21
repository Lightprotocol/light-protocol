# InvokeCpi

## Summary

| Field | Value |
|-------|-------|
| **Discriminator** | `[49, 212, 191, 129, 39, 194, 43, 196]` |
| **Enum** | `InstructionDiscriminator::InvokeCpi` |
| **Path** | `programs/system/src/invoke_cpi/` |

## Description

Processes compressed account state transitions via CPI (Cross-Program Invocation). This instruction is invoked by other programs (e.g., compressed-token program, custom Anchor programs) to execute compressed account operations on behalf of their users.

### Use Cases
- Compressed token transfers (invoked by compressed-token program)
- Custom program operations on compressed PDAs
- Multi-program transactions using CPI context
- Any program-owned compressed account state transitions

### Key Differences from Invoke
| Feature | Invoke | InvokeCpi |
|---------|--------|-----------|
| **Caller** | Direct user invocation | CPI from another program |
| **Signer check** | Authority owns inputs | Invoking program PDA owns inputs |
| **Data accounts** | Not allowed | Required for program-owned accounts |
| **CPI context** | Not supported | Supported for multi-program txs |

### State Changes
- Input compressed accounts are nullified
- Output compressed accounts are created
- CPI context can be written (first_set_context, set_context) or executed
- SOL can be compressed or decompressed

---

## Instruction Data

**Source:** `program-libs/compressed-account/src/instruction_data/zero_copy.rs`

```rust
pub struct ZInstructionDataInvokeCpi<'a> {
    pub proof: Option<Ref<'a, [u8], CompressedProof>>,
    pub input_root_indices: ZeroCopySlice<U16, u8>,
    pub new_address_params: ZeroCopySlice<ZPackedNewAddressParams, u8>,
    pub input_compressed_accounts_with_merkle_context:
        ZeroCopySlice<ZPackedCompressedAccountWithMerkleContext, u8>,
    pub output_compressed_accounts: ZeroCopySlice<ZPackedCompressedAccount, u8>,
    pub relay_fee: Option<Ref<'a, [u8], U64>>,
    pub compress_or_decompress_lamports: Option<Ref<'a, [u8], U64>>,
    pub is_compress: u8,
    pub cpi_context: Option<Ref<'a, [u8], CpiContext>>,
}
```

### Data Layout

```
[0..8]:    Discriminator (8 bytes)
[8..12]:   Vec length prefix (4 bytes, always skip in zero-copy parsing)
[12..]:    Zero-copy serialized instruction data (parsed by ZInstructionDataInvokeCpi)
```

**CPI-specific field:**
- `cpi_context`: Controls CPI context mode
  - `None`: Execute immediately without CPI context
  - `Some(CompressedCpiContext { first_set_context: true, set_context: false })`: First write
  - `Some(CompressedCpiContext { first_set_context: false, set_context: true })`: Subsequent write
  - `Some(CompressedCpiContext { first_set_context: false, set_context: false })`: Execute from context

---

## Accounts

| Index | Name | Signer | Writable | Description |
|-------|------|--------|----------|-------------|
| 0 | `fee_payer` | Yes | Yes | Pays transaction fees |
| 1 | `authority` | No | No | Authority (not necessarily signer in CPI) |
| 2 | `registered_program_pda` | No | No | Registered program PDA |
| 3 | `_unused` | No | No | Reserved slot (skipped during validation) |
| 4 | `account_compression_authority` | No | No | PDA for account-compression CPI |
| 5 | `account_compression_program` | No | No | Account Compression Program |
| 6 | `invoking_program` | No | No | Program making the CPI (verified as signer) |
| 7 | `sol_pool_pda` | No | Yes (conditional) | Sol pool PDA for compress/decompress |
| 8 | `decompression_recipient` | No | Yes (conditional) | Recipient for decompressed SOL |
| 9 | `system_program` | No | No | System Program |
| 10 | `cpi_context_account` | No | Yes (conditional) | CPI context account |
| 11+ | `remaining_accounts` | - | - | Merkle trees, queues |

### Account Validations

**authority (index 1):**
- Not required to be a signer (CPI mode)
- Must be a PDA derived from invoking_program with seeds `[CPI_AUTHORITY_PDA_SEED]`
- Proves the invoking program controls the authority

**invoking_program (index 6):**
- Must be the program making the CPI call
- Must match the authority PDA derivation

**cpi_context_account (index 10):**
- Required when `cpi_context` instruction data is present
- Must be owned by Light System Program
- Must have version 2 discriminator
- Must be associated with first Merkle tree

---

## Instruction Logic

### Step 1: Parse Instruction Data
```rust
let instruction_data = &instruction_data[4..];  // Skip 4-byte vec length prefix
let (inputs, _) = ZInstructionDataInvokeCpi::zero_copy_at(instruction_data)?;
```

### Step 2: Parse and Validate Accounts
```rust
let (ctx, remaining_accounts) = InvokeCpiInstruction::from_account_infos(accounts)?;
```

### Step 3: CPI Signer Checks
```rust
cpi_signer_checks::<T>(
    &invoking_program,
    accounts.get_authority().key(),
    &instruction_data,
)?;
```

**Three-part validation:**

1. **Authority PDA Check** - Derive PDA from invoking_program with seeds `[CPI_AUTHORITY_PDA_SEED]`, verify it matches authority
2. **Input Account Ownership Check** - For each input: verify owner == invoking_program
3. **Output Account Write Access Check** - For outputs with data: verify owner == invoking_program

### Step 4: Process CPI Context or Execute

**Write Mode (first_set_context or set_context):**
- Write/append instruction data to CPI context account
- Return early without executing

**Execute Mode (neither flag set):**
- Read data from CPI context account
- Combine with current instruction data
- Execute combined state transition with proof

**No CPI context:**
```rust
process::<true, InvokeCpiInstruction, ZInstructionDataInvokeCpi>(
    wrapped_inputs,
    None,
    &ctx,
    0,
    remaining_accounts,
)?;
```

---

## CPI Context Modes

| Mode | Flags | Behavior |
|------|-------|----------|
| **First Set** | `first_set_context = true` | Clear account, set fee payer, store data, return early |
| **Set Context** | `set_context = true` | Validate fee payer, append data, return early |
| **Execute** | Both `false` | Read all data, verify proof, execute, clear account |
| **No Context** | `cpi_context = None` | Execute immediately with current instruction data only |

See [CPI_CONTEXT.md](../CPI_CONTEXT.md) for detailed explanation.

---

## Errors

| Code | Error | Cause |
|------|-------|-------|
| - | `NotEnoughAccountKeys` | Less than 11 accounts provided |
| 6002 | `CpiSignerCheckFailed` | Invoking program doesn't match PDA signer |
| 6014 | `InvokingProgramNotProvided` | invoking_program account missing |
| 6020 | `CpiContextAccountUndefined` | CPI context data present but account missing |
| 6021 | `CpiContextEmpty` | CPI context account empty during execute |
| 6022 | `CpiContextMissing` | CPI context instruction data missing when expected |
| 6027 | `CpiContextFeePayerMismatch` | Fee payer doesn't match first invocation |
| 6028 | `CpiContextAssociatedMerkleTreeMismatch` | Wrong Merkle tree association |
| 6049 | `CpiContextAlreadySet` | Attempting to set when already set |
| 6054 | `CpiContextPassedAsSetContext` | Account doesn't exist but marked as set_context |

See [INSTRUCTIONS.md](../INSTRUCTIONS.md) for complete error list.

---

## Related Documentation

- [CPI_CONTEXT.md](../CPI_CONTEXT.md) - Detailed CPI context explanation
- [PROCESSING_PIPELINE.md](../PROCESSING_PIPELINE.md) - Processing flow
- [INVOKE.md](../invoke/INVOKE.md) - Direct invocation comparison
- [INVOKE_CPI_WITH_READ_ONLY.md](INVOKE_CPI_WITH_READ_ONLY.md) - With read-only support

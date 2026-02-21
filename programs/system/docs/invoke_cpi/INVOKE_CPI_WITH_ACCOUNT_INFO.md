# InvokeCpiWithAccountInfo

## Summary

| Field | Value |
|-------|-------|
| **Discriminator** | `[228, 34, 128, 84, 47, 139, 86, 240]` |
| **Enum** | `InstructionDiscriminator::InvokeCpiWithAccountInfo` |
| **Path** | `programs/system/src/lib.rs` |

## Description

Advanced CPI invocation instruction with dynamic account configuration. This instruction supports V2 account mode where the account list is determined by `AccountOptions` configuration flags embedded in the instruction data, enabling more flexible and efficient account passing.

### Use Cases
- Programs that want to minimize account overhead
- Dynamic account configurations based on operation type
- Advanced CPI scenarios with optional accounts
- Programs using non-Anchor account layouts

### Key Differences from Other Instructions

| Feature | InvokeCpi | InvokeCpiWithReadOnly | InvokeCpiWithAccountInfo |
|---------|-----------|------------------------|---------------------------|
| **Account layout** | Fixed (Anchor) | Anchor or V2 | V2 only |
| **Read-only** | No | Yes | Yes |
| **Configuration** | Implicit | Explicit (mode field) | Explicit (AccountOptions) |
| **Overhead** | Highest | Medium | Lowest |

### State Changes
- Input compressed accounts are nullified
- Output compressed accounts are created
- Read-only accounts are verified
- Read-only addresses are verified
- SOL can be compressed or decompressed

---

## Instruction Data

**Source:** `program-libs/compressed-account/src/instruction_data/with_account_info.rs`

```rust
pub struct InstructionDataInvokeCpiWithAccountInfo<'a> {
    pub invoking_program_id: Pubkey,
    pub mode: u8,  // Always 1 (V2 mode)
    pub account_option_config: AccountOptions,
    pub proof: Option<Ref<'a, [u8], CompressedProof>>,
    pub new_address_params: ZeroCopySlice<ZPackedNewAddressParams>,
    pub account_infos: Vec<CompressedAccountInfo>,
    pub relay_fee: Option<u64>,
    pub compress_or_decompress_lamports: Option<u64>,
    pub is_compress: bool,
    pub cpi_context: Option<CpiContext>,
    pub read_only_accounts: ZeroCopySlice<ZPackedReadOnlyCompressedAccount>,
    pub read_only_addresses: ZeroCopySlice<ZPackedReadOnlyAddress>,
}
```

### AccountOptions Configuration

```rust
pub struct AccountOptions {
    pub sol_pool_pda: bool,
    pub decompression_recipient: bool,
    pub cpi_context_account: bool,
    pub write_to_cpi_context: bool,
}
```

| Flag | When true | When false |
|------|-----------|------------|
| `write_to_cpi_context` | Omits all execution accounts (4 saved) | Includes execution accounts |
| `sol_pool_pda` | Includes sol_pool_pda | Omits (1 saved) |
| `decompression_recipient` | Includes recipient | Omits (1 saved) |
| `cpi_context_account` | Includes CPI context account | Omits (1 saved) |

---

## Accounts (V2 Mode)

### Base Accounts (Always Present)

| Index | Name | Signer | Writable | Description |
|-------|------|--------|----------|-------------|
| 0 | `fee_payer` | Yes | Yes | Pays transaction fees |
| 1 | `authority` | Yes | No | Authority signer |

### Conditional Execution Accounts

Present only when `write_to_cpi_context = false`:

| Name | Signer | Writable | Description |
|------|--------|----------|-------------|
| `registered_program_pda` | No | No | Registered program PDA |
| `account_compression_authority` | No | No | PDA for account-compression CPI |
| `account_compression_program` | No | No | Account Compression Program |
| `system_program` | No | No | System Program |

### Optional Accounts

| Name | Flag | Writable | Description |
|------|------|----------|-------------|
| `sol_pool_pda` | `sol_pool_pda` | Yes | Sol pool PDA |
| `decompression_recipient` | `decompression_recipient` | Yes | Recipient for decompressed SOL |
| `cpi_context_account` | `cpi_context_account` | Yes | CPI context account |

---

## Account Layout Examples

### Execute Mode (write_to_cpi_context = false)

**Full configuration (all flags true):**
```
[0] fee_payer (signer, writable)
[1] authority (signer)
[2] registered_program_pda
[3] account_compression_authority
[4] account_compression_program
[5] system_program
[6] sol_pool_pda
[7] decompression_recipient
[8] cpi_context_account
[9+] remaining_accounts
```

**Minimal configuration:**
```
[0] fee_payer
[1] authority
[2] registered_program_pda
[3] account_compression_authority
[4] account_compression_program
[5] system_program
[6+] remaining_accounts
```

### CPI Context Write Mode (write_to_cpi_context = true)

**Most compact configuration (3 accounts only):**
```
[0] fee_payer
[1] authority
[2] cpi_context_account
```

---

## Instruction Logic

### Step 1: Parse Instruction Data
```rust
let (inputs, _) = InstructionDataInvokeCpiWithAccountInfo::zero_copy_at(instruction_data)?;
```

### Step 2: Validate Mode
```rust
let mode = AccountMode::try_from(inputs.mode)?;
assert_eq!(mode, AccountMode::V2);  // V2 only
```

### Step 3: Parse Accounts with Dynamic Configuration
```rust
let (ctx, remaining_accounts) = InvokeCpiInstructionV2::from_account_infos(
    accounts,
    inputs.account_option_config,
)?;
```

The `AccountIterator` consumes accounts sequentially based on `AccountOptions` flags.

### Step 4: Process Invocation
```rust
process_invoke_cpi::<true, InvokeCpiInstructionV2, InstructionDataInvokeCpiWithAccountInfo>(
    invoking_program,
    ctx,
    inputs,
    remaining_accounts,
)?;
```

---

## AccountOptions Usage Patterns

### Pattern 1: Minimal (No Optional Accounts)
```rust
AccountOptions {
    write_to_cpi_context: false,
    sol_pool_pda: false,
    decompression_recipient: false,
    cpi_context_account: false,
}
// Accounts: 6 + remaining
```

### Pattern 2: SOL Compression
```rust
AccountOptions {
    write_to_cpi_context: false,
    sol_pool_pda: true,
    decompression_recipient: false,
    cpi_context_account: false,
}
// Accounts: 7 + remaining
```

### Pattern 3: SOL Decompression
```rust
AccountOptions {
    write_to_cpi_context: false,
    sol_pool_pda: true,
    decompression_recipient: true,
    cpi_context_account: false,
}
// Accounts: 8 + remaining
```

### Pattern 4: CPI Context Write
```rust
AccountOptions {
    write_to_cpi_context: true,
    sol_pool_pda: false,
    decompression_recipient: false,
    cpi_context_account: true,
}
// Accounts: 3 only (no remaining)
```

---

## Errors

| Code | Error | Cause |
|------|-------|-------|
| - | `NotEnoughAccountKeys` | Fewer accounts than configuration requires |
| 6044 | `InvalidAccountMode` | mode != 1 (V2) |
| 6054 | `CpiContextPassedAsSetContext` | write_to_cpi_context but no CPI context account |
| 6057 | `InvalidAccountIndex` | Account index out of bounds during parsing |

See [INSTRUCTIONS.md](../INSTRUCTIONS.md) for complete error list.

---

## Related Documentation

- [INVOKE_CPI.md](INVOKE_CPI.md) - Base CPI invocation (Anchor mode)
- [INVOKE_CPI_WITH_READ_ONLY.md](INVOKE_CPI_WITH_READ_ONLY.md) - Read-only support
- [CPI_CONTEXT.md](../CPI_CONTEXT.md) - CPI context usage
- [PROCESSING_PIPELINE.md](../PROCESSING_PIPELINE.md) - Processing flow

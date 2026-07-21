# Invoke

## Summary

| Field | Value |
|-------|-------|
| **Discriminator** | `[26, 16, 169, 7, 21, 202, 242, 25]` |
| **Enum** | `InstructionDiscriminator::Invoke` |
| **Path** | `programs/system/src/invoke/` |

## Description

Processes compressed account state transitions for a single program (no CPI). This is the direct invocation mode where the authority directly signs for all input compressed accounts without delegating to another program.

### Use Cases
- Transferring compressed SOL between accounts
- Creating/closing compressed accounts owned by a single authority
- Direct state transitions without multi-program coordination

### Constraints
- All input compressed accounts must be owned by the authority (signer)
- Input accounts CANNOT have data (only programs can own accounts with data)
- Cannot be used for multi-program transactions (use InvokeCpi for that)

### Key Differences from InvokeCpi

| Feature | Invoke | InvokeCpi |
|---------|--------|-----------|
| **Caller** | Direct user invocation | CPI from another program |
| **Authority signer** | Must be a signer | Not required to be a signer |
| **Signer check** | Authority must own all inputs | Invoking program PDA must own inputs |
| **Data accounts** | Input accounts CANNOT have data | Program-owned accounts CAN have data |
| **CPI context** | Not supported | Supported for multi-program txs |
| **Use case** | Compressed SOL transfers, user-owned accounts | Token transfers, custom program PDAs |

### State Changes
- Input compressed accounts are nullified (inserted into nullifier queue)
- Output compressed accounts are created (inserted into output queue)
- New addresses are created (inserted into address queue)
- SOL can be compressed or decompressed

---

## Instruction Data

**Source:** `program-libs/compressed-account/src/instruction_data/zero_copy.rs`

```rust
pub struct ZInstructionDataInvoke<'a> {
    pub proof: Option<Ref<&'a [u8], CompressedProof>>,
    pub input_compressed_accounts_with_merkle_context:
        Vec<ZPackedCompressedAccountWithMerkleContext<'a>>,
    pub output_compressed_accounts: Vec<ZOutputCompressedAccountWithPackedContext<'a>>,
    pub relay_fee: Option<Ref<&'a [u8], U64>>,
    pub new_address_params: ZeroCopySliceBorsh<'a, ZNewAddressParamsPacked>,
    pub compress_or_decompress_lamports: Option<Ref<&'a [u8], U64>>,
    pub is_compress: bool,
}
```

### Data Layout

```
[0..8]:    Discriminator
[8..12]:   Vec length prefix (4 bytes, always skip)
[12..]:    Serialized instruction data
```

**Key fields:**
- `proof`: Optional ZK proof (compressed, ~128 bytes when present)
- `input_compressed_accounts_with_merkle_context`: Accounts to nullify (with Merkle context for verification)
- `output_compressed_accounts`: Accounts to create (with packed context)
- `relay_fee`: Optional relay fee amount
- `new_address_params`: Parameters for creating new addresses
- `compress_or_decompress_lamports`: Amount of SOL to compress/decompress
- `is_compress`: true = compress SOL, false = decompress SOL

---

## Accounts

| Index | Name | Signer | Writable | Description |
|-------|------|--------|----------|-------------|
| 0 | `fee_payer` | Yes | Yes | Pays transaction fees, rollover fees, protocol fees |
| 1 | `authority` | Yes | No | Signs for all input compressed accounts |
| 2 | `registered_program_pda` | No | No | Registered program PDA (checked in account-compression) |
| 3 | `_unused` | No | No | Backwards compatibility slot (previously noop program) |
| 4 | `account_compression_authority` | No | No | PDA authority for account-compression CPI |
| 5 | `account_compression_program` | No | No | Account Compression Program |
| 6 | `sol_pool_pda` | No | Yes (conditional) | Sol pool PDA for compress/decompress operations |
| 7 | `decompression_recipient` | No | Yes (conditional) | Recipient account for decompressed SOL |
| 8 | `system_program` | No | No | System Program (for SOL transfers) |
| 9+ | `remaining_accounts` | - | - | Merkle trees, queues, and other dynamic accounts |

### Account Validations

**fee_payer (index 0):**
- Must be a signer
- Must be writable
- Pays for all transaction fees

**authority (index 1):**
- Must be a signer
- Can be the same account as fee_payer
- Must match the owner of all input compressed accounts

**sol_pool_pda (index 6):**
- Required when `compress_or_decompress_lamports` is set
- Optional (can be system program placeholder) otherwise

**decompression_recipient (index 7):**
- Required when decompressing SOL (`is_compress = 0`)
- Must be writable

### Remaining Accounts

The remaining accounts array contains Merkle trees and queues in a specific order:

1. **State Merkle trees** (1 per unique tree in inputs)
2. **State output queues** (1 per unique tree in outputs)
3. **Address Merkle trees** (1 per new address)
4. **Address queues** (1 per new address)

---

## Instruction Logic

### Step 1: Parse Instruction Data
```rust
let instruction_data = &instruction_data[4..];  // Skip vec prefix
let (inputs, _) = ZInstructionDataInvoke::zero_copy_at(instruction_data)?;
```

### Step 2: Parse and Validate Accounts
```rust
let (ctx, remaining_accounts) = InvokeInstruction::from_account_infos(accounts)?;
```

### Step 3: Verify Authority Signature
```rust
input_compressed_accounts_signer_check(
    &inputs.input_compressed_accounts_with_merkle_context,
    ctx.authority.key(),
)?;
```

**For each input compressed account:**
- Account owner must equal authority pubkey
- Account must NOT have data (`data.is_none()`)

### Step 4: Process State Transition
```rust
process::<false, InvokeInstruction, ZInstructionDataInvoke>(
    wrapped_inputs,
    None,              // No CPI context
    &ctx,
    0,                 // Default relay fee
    remaining_accounts,
)?;
```

Executes the full 19-step processing pipeline (see [PROCESSING_PIPELINE.md](../PROCESSING_PIPELINE.md)).

---

## Errors

| Code | Error | Cause |
|------|-------|-------|
| - | `NotEnoughAccountKeys` | Less than 9 accounts provided |
| - | `MissingRequiredSignature` | fee_payer or authority not a signer |
| - | `IncorrectProgramId` | account_compression_program or system_program wrong |
| 6000 | `SumCheckFailed` | Lamport conservation violated |
| 6001 | `SignerCheckFailed` | Authority doesn't own input account OR account has data |
| 6008 | `CompressedSolPdaUndefinedForCompressSol` | sol_pool_pda missing when compressing |
| 6009 | `DecompressLamportsUndefinedForCompressSol` | compress_or_decompress_lamports missing |
| 6010 | `CompressedSolPdaUndefinedForDecompressSol` | sol_pool_pda missing when decompressing |
| 6011 | `DeCompressLamportsUndefinedForDecompressSol` | compress_or_decompress_lamports missing |
| 6012 | `DecompressRecipientUndefinedForDecompressSol` | decompression_recipient missing when decompressing |
| 6017 | `ProofIsNone` | ZK proof required but not provided |
| 6019 | `EmptyInputs` | No inputs, outputs, or addresses provided |
| 6043 | `ProofVerificationFailed` | ZK proof verification failed |

See [INSTRUCTIONS.md](../INSTRUCTIONS.md) for complete error list.

---

## Related Documentation

- [PROCESSING_PIPELINE.md](../PROCESSING_PIPELINE.md) - Detailed 19-step processing flow
- [INSTRUCTIONS.md](../INSTRUCTIONS.md) - Complete error reference
- [InvokeCpi](../invoke_cpi/INVOKE_CPI.md) - CPI invocation mode (Anchor layout)

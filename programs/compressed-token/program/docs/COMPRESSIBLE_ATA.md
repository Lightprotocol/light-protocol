# Compressible ATA Lifecycle

This document describes the complete lifecycle of a compressible Associated Token Account (ATA) from creation through compress-and-close to decompression.

## Overview

A **compressible ATA** is an Associated Token Account with the Compressible extension enabled. Unlike regular ATAs or regular CToken accounts, compressible ATAs support rent management and can be compressed to save on-chain storage costs.

**Key characteristics:**
- PDA derived from `[wallet_owner, ctoken_program_id, mint, bump]`
- Always has `compression_only` flag set (required)
- Cannot use `compress_to_pubkey` (ATAs always compress to owner)
- Has `is_ata=1` flag in Compressible extension
- Supports full state preservation during compress/decompress cycles

**Why `is_ata` matters:**
ATAs are PDAs and cannot sign transactions. During compression, the ATA pubkey becomes the "owner" of the compressed token account. The `is_ata` flag and associated `owner_index`/`bump` fields allow the system to:
1. Validate the original wallet owner during decompress
2. Verify ATA derivation to prevent spoofing
3. Route signing authority to the wallet owner instead of the ATA

---

## Data Structures

### CompressibleExtension (on CToken account)

**Path:** `program-libs/ctoken-interface/src/state/extensions/compressible.rs`

```rust
pub struct CompressibleExtension {
    pub decimals_option: u8,       // Whether decimals are cached
    pub decimals: u8,              // Cached decimals value
    pub compression_only: bool,    // Must be true for ATAs
    pub is_ata: u8,                // 1=ATA, 0=regular account
    pub info: CompressionInfo,     // Rent management data
}
```

The `is_ata` field is set to `1` during ATA creation and is used during CompressAndClose to determine owner validation behavior.

### CompressedOnlyExtension (on compressed account TLV)

**Path:** `program-libs/ctoken-interface/src/state/extensions/compressed_only.rs`

```rust
pub struct CompressedOnlyExtension {
    pub delegated_amount: u64,         // Preserved delegated amount
    pub withheld_transfer_fee: u64,    // Preserved withheld fees
    pub is_ata: u8,                    // ATA flag (1 or 0)
}
```

This extension is stored in the compressed token account's TLV data and preserves account state during compression.

### CompressedOnlyExtensionInstructionData (passed in instruction)

**Path:** `program-libs/ctoken-interface/src/instructions/extensions/compressed_only.rs`

```rust
pub struct CompressedOnlyExtensionInstructionData {
    pub delegated_amount: u64,
    pub withheld_transfer_fee: u64,
    pub is_frozen: bool,           // Whether source was frozen
    pub compression_index: u8,     // Index of compression operation
    pub is_ata: bool,              // ATA flag
    pub bump: u8,                  // ATA PDA derivation bump
    pub owner_index: u8,           // Index to wallet owner in packed accounts
}
```

The `bump` and `owner_index` fields are only used when `is_ata=true` to verify ATA derivation during decompress.

---

## 1. ATA Creation

**Path:** `programs/compressed-token/program/src/ctoken/create_ata.rs`

### Instruction: CreateAssociatedCTokenAccount (100) / CreateAssociatedTokenAccountIdempotent (102)

### Accounts
1. associated_token_account (mutable) - The ATA to create
2. fee_payer (signer, mutable) - Pays transaction fees
3. owner - Wallet owner of the ATA
4. mint - Token mint account
5. system_program
6. compressible_config (optional) - Required for compressible ATAs
7. rent_payer (optional) - Custom rent payer or uses config.rent_sponsor

### Validation Checks

| Check | Error |
|-------|-------|
| Account owned by system program (uninitialized) | `IllegalOwner` |
| `compress_to_account_pubkey` is None | `InvalidInstructionData` |
| `compression_only != 0` | `AtaRequiresCompressionOnly` |
| CompressibleConfig is ACTIVE | `InvalidState` |
| `rent_payment != 1` epoch | `OneEpochPrefundingNotAllowed` |
| PDA derivation correct (idempotent mode) | Validated by `validate_ata_derivation` |

### Creation Flow

1. **Derive ATA address:**
   ```rust
   let (ata_pubkey, bump) = Pubkey::find_program_address(
       &[owner.key(), CTOKEN_PROGRAM_ID.as_ref(), mint.key()],
       &CTOKEN_PROGRAM_ID
   );
   ```

2. **Validate compressible requirements:**
   - `compression_only` must be non-zero
   - `compress_to_account_pubkey` must be None (ATAs compress to owner automatically)

3. **Calculate account size:** Includes Compressible extension and any mint extension markers

4. **Calculate rent:** rent_exemption + prepaid_epochs_rent + compression_incentive

5. **Create account:** Via CPI with rent_sponsor PDA or custom rent payer

6. **Initialize CToken:** Sets `is_ata=1` in Compressible extension

**Path:** `programs/compressed-token/program/src/shared/initialize_ctoken_account.rs` (line 300)
```rust
compressible_ext.is_ata = is_ata as u8;  // Set from create_ata parameter
```

---

## 2. CompressAndClose

**Path:** `programs/compressed-token/program/src/compressed_token/transfer2/compression/ctoken/compress_and_close.rs`

### Instruction: Transfer2 (101) with CompressionMode::CompressAndClose

CompressAndClose closes a CToken account and creates a compressed token account. For ATAs, this requires special handling because the ATA pubkey becomes the owner of the compressed account.

### Owner Validation (lines 103-115)

For ATAs, the expected owner is the **ATA pubkey** (not the wallet owner):

```rust
let expected_owner = if compression.info.compress_to_pubkey() || compression.is_ata() {
    token_account_pubkey  // ATA pubkey is the owner
} else {
    &ctoken.owner.to_bytes()
};
if output_owner != expected_owner {
    return Err(ErrorCode::CompressAndCloseInvalidOwner.into());
}
```

### CompressedOnly Extension Requirement (lines 136-145)

ATAs **require** the CompressedOnly extension in output TLV:

```rust
if (compression.compression_only() || compression.is_ata()) && compression_only_ext.is_none() {
    return Err(ErrorCode::CompressAndCloseMissingCompressedOnlyExtension.into());
}
```

### Data Preservation Validation (lines 170-222)

The `validate_compressed_only_ext` function validates all preserved data:

| Field | Validation | Error |
|-------|------------|-------|
| `delegated_amount` | Must match CToken's delegated_amount | `CompressAndCloseDelegatedAmountMismatch` (6135) |
| `delegate` | Must match if delegated_amount > 0 | `CompressAndCloseInvalidDelegate` (6136) |
| `withheld_transfer_fee` | Must match TransferFeeAccount withheld | `CompressAndCloseWithheldFeeMismatch` (6137) |
| `is_frozen` | Must match CToken state (state == 2) | `CompressAndCloseFrozenMismatch` (6138) |
| `is_ata` | Must match Compressible.is_ata | `CompressAndCloseIsAtaMismatch` (6168) |

```rust
// is_ata validation (lines 216-219)
if compression.is_ata() != ext.is_ata() {
    return Err(ErrorCode::CompressAndCloseIsAtaMismatch.into());
}
```

### CToken Reset After Compression (lines 68-71)

```rust
ctoken.base.amount.set(0);
// Unfreeze the account if frozen (frozen state is preserved in compressed token TLV)
// This allows the close_token_account validation to pass for frozen accounts
ctoken.base.set_initialized();
```

### Additional Validation

- **Amount:** compression_amount == output_amount == ctoken.amount
- **Mint:** output mint matches ctoken mint
- **Version:** Must be ShaFlat (version=3)
- **Uniqueness:** Each CompressAndClose must use different compressed output indices

---

## 3. Decompress

**Path:** `programs/compressed-token/program/src/compressed_token/transfer2/compression/ctoken/decompress.rs`

### Instruction: Transfer2 (101) with CompressionMode::Decompress

Decompress recreates a CToken account from a compressed token account. For ATAs, the system must verify the ATA derivation and restore the wallet owner.

### Amount Validation (lines 31-46)

For ATAs (and compress_to_pubkey), the amount **must match exactly**:

```rust
if ext_data.is_ata() || compress_to_pubkey {
    let input_amount: u64 = inputs.input_token_data.amount.into();
    if compression_amount != input_amount {
        return Err(CTokenError::DecompressAmountMismatch.into());
    }
}
```

This prevents partial decompression of ATA tokens, ensuring the full balance is always decompressed together.

### Destination Validation (lines 77-106)

For ATAs, validation is more complex because of the owner paradox:

```rust
fn validate_destination(
    ctoken: &ZCTokenMut,
    destination: &AccountInfo,
    input_owner_key: &[u8; 32],
    ext_data: &ZCompressedOnlyExtensionInstructionData,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
) -> Result<(), ProgramError> {
    // Non-ATA: simple owner match
    if !ext_data.is_ata() {
        if !pubkey_eq(ctoken.base.owner.array_ref(), input_owner_key) {
            return Err(CTokenError::DecompressDestinationMismatch.into());
        }
        return Ok(());
    }

    // ATA: destination address == input_owner (ATA pubkey)
    if !pubkey_eq(destination.key(), input_owner_key) {
        return Err(CTokenError::DecompressDestinationMismatch.into());
    }

    // ATA: wallet owner from owner_index must match CToken owner
    let wallet = packed_accounts.get_u8(ext_data.owner_index, "wallet owner")?;
    if !pubkey_eq(wallet.key(), ctoken.base.owner.array_ref()) {
        return Err(CTokenError::DecompressDestinationMismatch.into());
    }
    Ok(())
}
```

### ATA Derivation Verification (Critical Security Check)

**Path:** `programs/compressed-token/program/src/shared/token_input.rs` (lines 82-120)

During **input processing** (before decompress.rs runs), the ATA derivation is verified:

```rust
// For ATA decompress, verify wallet owner and ATA derivation
if data.is_ata != 0 {
    // 1. Get wallet owner from owner_index
    let wallet_owner = packed_accounts.get(data.owner_index as usize)?;

    // 2. Derive ATA using bump from CompressedOnly extension
    let bump_seed = [data.bump];
    let ata_seeds: [&[u8]; 4] = [
        wallet_owner.key().as_ref(),
        CTOKEN_PROGRAM_ID.as_ref(),
        mint_account.key().as_ref(),
        bump_seed.as_ref(),
    ];
    let derived_ata = create_program_address(&ata_seeds, &CTOKEN_PROGRAM_ID)?;

    // 3. Verify owner_account (the ATA pubkey) matches derived address
    if !pubkey_eq(owner_account.key(), &derived_ata) {
        return None; // Causes signer check to fail
    }

    // 4. Use wallet owner as signer (ATA can't sign)
    signer_account = wallet_owner;
}

// 5. verify_owner_or_delegate_signer validates wallet_owner is a transaction signer
```

**Security Properties:**
- Incorrect `owner_index`: ATA derivation fails or wallet_owner is wrong, signer check fails
- Incorrect `bump`: ATA derivation produces wrong address, validation fails
- Malicious wallet: Not a transaction signer, `verify_owner_or_delegate_signer` fails

### State Restoration

1. **Delegate restoration (lines 108-144):**
   ```rust
   if let Some(delegate_acc) = input_delegate {
       ctoken.base.set_delegate(Some(Pubkey::from(*delegate_acc.key())))?;
       if delegated_amount > 0 {
           ctoken.base.delegated_amount.set(current + delegated_amount);
       }
   }
   ```

2. **Withheld fee restoration (lines 146-171):**
   ```rust
   if fee > 0 {
       let fee_ext = ctoken.get_transfer_fee_account_extension_mut();
       fee_ext.add_withheld_amount(fee)?;
   }
   ```

3. **Frozen state restoration (lines 64-67):**
   ```rust
   if ext_data.is_frozen() {
       ctoken.base.set_frozen();
   }
   ```

---

## Validation Summary

### Creation Validations

| Validation | Source | Error |
|------------|--------|-------|
| Account uninitialized | create_ata.rs | `IllegalOwner` |
| compression_only set | create_ata.rs | `AtaRequiresCompressionOnly` |
| compress_to_pubkey is None | create_ata.rs | `InvalidInstructionData` |
| Config is ACTIVE | create_ata.rs | `InvalidState` |
| rent_payment != 1 | initialize_ctoken_account.rs | `OneEpochPrefundingNotAllowed` |

### CompressAndClose Validations

| Validation | Source | Error |
|------------|--------|-------|
| Owner matches (ATA pubkey) | compress_and_close.rs:103-115 | `CompressAndCloseInvalidOwner` |
| CompressedOnly extension present | compress_and_close.rs:136-145 | `CompressAndCloseMissingCompressedOnlyExtension` (6133) |
| delegated_amount matches | compress_and_close.rs:177-181 | `CompressAndCloseDelegatedAmountMismatch` (6135) |
| delegate pubkey matches | compress_and_close.rs:183-194 | `CompressAndCloseInvalidDelegate` (6136) |
| withheld_transfer_fee matches | compress_and_close.rs:196-209 | `CompressAndCloseWithheldFeeMismatch` (6137) |
| is_frozen matches | compress_and_close.rs:211-214 | `CompressAndCloseFrozenMismatch` (6138) |
| is_ata matches | compress_and_close.rs:216-219 | `CompressAndCloseIsAtaMismatch` (6168) |
| Amount matches | compress_and_close.rs:117-121 | `CompressAndCloseAmountMismatch` |
| Mint matches | compress_and_close.rs:123-129 | `CompressAndCloseInvalidMint` |
| Version is ShaFlat | compress_and_close.rs:131-134 | `CompressAndCloseInvalidVersion` |

### Decompress Validations

| Validation | Source | Error |
|------------|--------|-------|
| Amount matches (for ATA) | decompress.rs:31-46 | `DecompressAmountMismatch` (18064) |
| Destination = input_owner (ATA pubkey) | decompress.rs:89-93 | `DecompressDestinationMismatch` (18057) |
| Wallet owner = CToken owner | decompress.rs:96-102 | `DecompressDestinationMismatch` (18057) |
| ATA derivation correct | token_input.rs:79-117 | Error during input processing |
| Delegate present if delegated_amount > 0 | decompress.rs:139-142 | `DecompressDelegatedAmountWithoutDelegate` (18059) |
| TransferFeeAccount ext if withheld_fee > 0 | decompress.rs:160-168 | `DecompressWithheldFeeWithoutExtension` (18060) |

---

## Data Preservation Matrix

| Field | Preserved | Storage Location | Restored | Notes |
|-------|-----------|------------------|----------|-------|
| is_ata | Yes | CompressedOnly.is_ata | Validated | Must match source |
| delegated_amount | Yes | CompressedOnly.delegated_amount | Yes | Restored to CToken |
| withheld_transfer_fee | Yes | CompressedOnly.withheld_transfer_fee | Yes | Restored to TransferFeeAccount ext |
| is_frozen | Yes | CompressedOnly.is_frozen | Yes | Restored via set_frozen() |
| bump | Yes | CompressedOnly.bump | Used | For ATA derivation verification |
| owner_index | Yes | CompressedOnly.owner_index | Used | Identifies wallet owner account |
| delegate pubkey | Yes | Passed as input account | Yes | Restored to CToken.delegate |
| amount | No | New from compression | N/A | Set to 0 after compress |
| close_authority | No | Not preserved | N/A | Cannot be set on ATAs anyway |

---

## Security: Why is_ata Flag is Trustworthy

The `is_ata` flag in the Compressible extension is **program-controlled** and cannot be spoofed:

1. **Set during creation only:** The flag is set by `create_ata.rs` when creating an ATA, or `create.rs` for regular accounts
2. **Account owned by program:** CToken accounts are owned by the CToken program, preventing external modification
3. **Validated during CompressAndClose:** The `is_ata` in Compressible extension must match CompressedOnly extension (line 217)

**Attack prevention:**
- Cannot create non-ATA with `is_ata=1`: Program controls flag during creation
- Cannot modify existing account's flag: Account is program-owned
- Cannot spoof in CompressedOnly: Must match Compressible extension

---

## ATA Owner Paradox

ATAs present a unique challenge because they are PDAs and **cannot sign transactions**. This creates a paradox during compression:

1. **The compressed token owner must be verifiable** - so we use the ATA pubkey as owner
2. **Someone must sign the decompress transaction** - but the ATA can't sign

**Solution:**

The CompressedOnly extension stores:
- `owner_index`: Index to the wallet owner account in packed_accounts
- `bump`: The PDA bump for ATA derivation

During decompress:
1. Wallet owner (from `owner_index`) provides the signature
2. System verifies: `derive_ata(wallet_owner, mint, bump) == input_owner`
3. This proves the wallet owner is the legitimate owner of the ATA

```
Creation:
  wallet_owner + mint + bump -> ATA pubkey
                                   |
                                   v
Compress:                    [ATA as owner]
                                   |
                                   v
Decompress:                  [verify derivation]
  wallet_owner (signer) <-------- owner_index
  mint                   <-------- from input
  bump                   <-------- from CompressedOnly
                                   |
                                   v
                            derived == input_owner?
```

---

## Error Reference

| Error | Code | Description |
|-------|------|-------------|
| `AtaRequiresCompressionOnly` | - | ATA created without compression_only flag |
| `CompressAndCloseMissingCompressedOnlyExtension` | 6133 | ATA CompressAndClose missing required extension |
| `CompressAndCloseIsAtaMismatch` | 6168 | is_ata flag mismatch between extensions |
| `CompressAndCloseInvalidOwner` | 6089 | Owner validation failed (ATA pubkey expected) |
| `CompressAndCloseDelegatedAmountMismatch` | 6135 | Delegated amount not preserved correctly |
| `CompressAndCloseInvalidDelegate` | 6136 | Delegate pubkey mismatch |
| `CompressAndCloseWithheldFeeMismatch` | 6137 | Withheld fee not preserved correctly |
| `CompressAndCloseFrozenMismatch` | 6138 | Frozen state not preserved correctly |
| `DecompressDestinationMismatch` | 18057 | Destination/owner validation failed |
| `DecompressAmountMismatch` | 18064 | Amount mismatch for ATA decompress |
| `DecompressDelegatedAmountWithoutDelegate` | 18059 | delegated_amount > 0 but no delegate account |
| `DecompressWithheldFeeWithoutExtension` | 18060 | Withheld fee but no TransferFeeAccount extension |

---

## Related Documentation

- `docs/ctoken/CREATE.md` - Full ATA creation documentation
- `docs/compressed_token/TRANSFER2.md` - Transfer2 instruction including compress/decompress
- `docs/EXTENSIONS.md` - CompressedOnly extension behavior and validation
- `program-libs/compressible/docs/RENT.md` - Rent management for compressible accounts

# Token-2022 Extensions

This document describes how Token-2022 extensions are validated across compressed token instructions.

## Overview

The compressed token program supports 16 Token-2022 extension types. **5 restricted extensions** require instruction-level validation checks. Pure mint extensions (metadata, group, etc.) are allowed without explicit instruction support.

**Allowed extensions** (defined in `program-libs/ctoken-interface/src/token_2022_extensions.rs:17-44`):

1. MetadataPointer
2. TokenMetadata
3. InterestBearingConfig
4. GroupPointer
5. GroupMemberPointer
6. TokenGroup
7. TokenGroupMember
8. MintCloseAuthority
9. TransferFeeConfig *(restricted)*
10. DefaultAccountState *(restricted)*
11. PermanentDelegate *(restricted)*
12. TransferHook *(restricted)*
13. Pausable *(restricted)*
14. ConfidentialTransferMint
15. ConfidentialTransferFeeConfig
16. ConfidentialMintBurn

**Restricted extensions** require `compression_only` mode when creating token accounts, and have runtime checks during transfers.
- restricted extensions are only supported in ctoken accounts not compressed accounts.
- compression only prevents compressed transfers once ctoken accounts are compressed and closed.

## Quick Reference

| Instruction              | TransferFee       | DefaultState       | PermanentDelegate  | TransferHook      | Pausable           |
|--------------------------|-------------------|--------------------|--------------------|-------------------|--------------------|
| CreateTokenAccount       | requires comp_only| applies frozen     | requires comp_only | requires comp_only| requires comp_only |
| Transfer2 (→compressed)  | blocked           | -                  | blocked            | blocked           | blocked if paused  |
| Transfer2 (c→c)          | blocked           | -                  | blocked            | blocked           | blocked            |
| Transfer2 (SPL→CToken)   | fees must be 0    | -                  | -                  | hook must be nil  | blocked if paused  |
| Transfer2 (CToken→SPL)   | fees must be 0    | -                  | -                  | hook must be nil  | blocked if paused  |
| Transfer2 (decompress)   | allowed           | restores frozen    | allowed            | allowed           | allowed            |
| Transfer2 (C&C)          | allowed           | preserved          | allowed            | allowed           | allowed            |
| CTokenTransfer           | fees must be 0    | frozen blocked     | authority check    | hook must be nil  | blocked if paused  |
| CTokenApprove            | -                 | frozen blocked     | -                  | -                 | -                  |
| CTokenRevoke             | -                 | frozen blocked     | -                  | -                 | -                  |
| CTokenBurn               | N/A (CMint-only)  | frozen blocked     | N/A (CMint-only)   | N/A (CMint-only)  | N/A (CMint-only)   |
| CTokenMintTo             | N/A (CMint-only)  | -                  | N/A (CMint-only)   | N/A (CMint-only)  | N/A (CMint-only)   |
| CTokenFreeze/Thaw        | -                 | -                  | -                  | -                 | -                  |
| CloseTokenAccount        | -                 | -                  | -                  | -                 | -                  |
| CreateTokenPool          | fees must be 0    | -                  | -                  | hook must be nil  | -                  |

**Transfer2 Mode Definitions:**
- `→compressed` = Compress to output compressed account (Compress mode with compressed outputs)
- `c→c` = Transfer between compressed accounts
- `SPL→CToken` = Transfer from SPL token account to CToken account (uses Compress mode)
- `CToken→SPL` = Transfer from CToken account to SPL token account (uses Compress+Decompress)
- `decompress` = Decompress from compressed account to SPL/CToken (pure Decompress, no Compress)
- `C&C` = CompressAndClose mode

**Key:**
- `requires comp_only` = Extension triggers compression_only requirement with CompressionOnlyRequired (6131)
- `blocked` = Operation fails with MintHasRestrictedExtensions (6142)
- `fees must be 0` / `hook must be nil` = Specific validation check (errors: 6129, 6130)
- `blocked if paused` = Fails with MintPaused (6127) when mint is paused
- `frozen blocked` = Account frozen state prevents operation (pinocchio check)
- `allowed` = Bypasses extension state checks (decompress/C&C exit paths)
- `N/A (CMint-only)` = Instruction only works with CMints which don't support restricted extensions
- `-` = No extension-specific behavior

---

## Restricted Extensions

### 1. TransferFeeConfig

**Constraint:** Both `older_transfer_fee` and `newer_transfer_fee` must have `transfer_fee_basis_points == 0` and `maximum_fee == 0`.

| Instruction | Validation Function | Check | Error |
|-------------|---------------------|-------|-------|
| CreateTokenPool | `assert_mint_extensions()` | Fees must be zero | `NonZeroTransferFeeNotSupported` (6129) |
| Transfer2 | `check_mint_extensions()` | Fees must be zero | `NonZeroTransferFeeNotSupported` (6129) |
| CTokenTransfer | `check_mint_extensions()` | Fees must be zero | `NonZeroTransferFeeNotSupported` (6129) |
| CreateTokenAccount | `has_mint_extensions()` | Flags restricted extension | `CompressionOnlyRequired` (6131) |

**Validation paths:**
- `programs/compressed-token/anchor/src/instructions/create_token_pool.rs:142-153` - `assert_mint_extensions()` checks TransferFeeConfig
- `programs/compressed-token/program/src/extensions/check_mint_extensions.rs:86-99` - `parse_mint_extensions()` checks TransferFeeConfig (lines 86-99 in file)

**Unchecked instructions:**
1. CTokenApprove
2. CTokenRevoke
3. CTokenBurn
4. CTokenMintTo
5. CTokenFreezeAccount
6. CTokenThawAccount
7. CloseTokenAccount

---

### 2. TransferHook

**Constraint:** `program_id` must be nil (no active hook program).

| Instruction | Validation Function | Check | Error |
|-------------|---------------------|-------|-------|
| CreateTokenPool | `assert_mint_extensions()` | program_id must be nil | `TransferHookNotSupported` (6130) |
| Transfer2 | `check_mint_extensions()` | program_id must be nil | `TransferHookNotSupported` (6130) |
| CTokenTransfer | `check_mint_extensions()` | program_id must be nil | `TransferHookNotSupported` (6130) |
| CreateTokenAccount | `has_mint_extensions()` | Flags restricted extension | `CompressionOnlyRequired` (6131) |

**Validation paths:**
- `programs/compressed-token/anchor/src/instructions/create_token_pool.rs:155-162` - `assert_mint_extensions()` checks TransferHook
- `programs/compressed-token/program/src/extensions/check_mint_extensions.rs:101-107` - `parse_mint_extensions()` checks TransferHook (lines 101-107 in file)

**Unchecked instructions:**
1. CTokenApprove
2. CTokenRevoke
3. CTokenBurn
4. CTokenMintTo
5. CTokenFreezeAccount
6. CTokenThawAccount
7. CloseTokenAccount

---

### 3. PermanentDelegate

**Behavior:** Permanent delegate can authorize transfers/burns in addition to owner.

| Instruction | Validation Function | Check | Error |
|-------------|---------------------|-------|-------|
| CreateTokenAccount | `has_mint_extensions()` | Flags restricted extension | `CompressionOnlyRequired` (6131) |
| Transfer2 | `parse_mint_extensions()` → `verify_owner_or_delegate_signer()` | Extract delegate pubkey, then validate authority is owner OR delegate. If authority matches permanent delegate, that account must be a signer. | `OwnerMismatch` (6075) or `MissingRequiredSignature` |
| CTokenTransfer | `parse_mint_extensions()` → `verify_owner_or_delegate_signer()` | Extract delegate pubkey, then validate authority is owner OR delegate. If authority matches permanent delegate, that account must be a signer. | `OwnerMismatch` (6075) or `MissingRequiredSignature` |

**Validation paths:**
- `programs/compressed-token/program/src/extensions/check_mint_extensions.rs:76-84` - Extracts delegate pubkey in `parse_mint_extensions()`
- `programs/compressed-token/program/src/shared/owner_validation.rs:30-78` - `verify_owner_or_delegate_signer()` validates delegate/permanent delegate signer
- `programs/compressed-token/program/src/ctoken/transfer/shared.rs:196-214` - `validate_permanent_delegate()`

**Unchecked instructions:**
1. CTokenApprove
2. CTokenRevoke
3. CTokenBurn - permanent delegate cannot burn without owner signature
4. CTokenMintTo
5. CTokenFreezeAccount
6. CTokenThawAccount
7. CloseTokenAccount

---

### 4. Pausable

**Constraint:** If `pausable_config.paused == true`, all transfer operations fail immediately.

| Instruction | Validation Function | Check | Error |
|-------------|---------------------|-------|-------|
| CreateTokenAccount | `has_mint_extensions()` | Flags restricted extension | `CompressionOnlyRequired` (6131) |
| Transfer2 | `check_mint_extensions()` | `pausable_config.paused == false` | `MintPaused` (6127) |
| CTokenTransfer | `check_mint_extensions()` | `pausable_config.paused == false` | `MintPaused` (6127) |

**Validation path:**
- `programs/compressed-token/program/src/extensions/check_mint_extensions.rs:70-74` - `parse_mint_extensions()` checks PausableConfig.paused
- `programs/compressed-token/program/src/extensions/check_mint_extensions.rs:147-150` - `check_mint_extensions()` throws MintPaused error

**Unchecked instructions:**
1. CTokenApprove - allowed when paused (only affects delegation, not token movement)
2. CTokenRevoke - allowed when paused (only affects delegation, not token movement)
3. CTokenBurn - N/A (CMint-only instruction, CMints don't support Pausable)
4. CTokenMintTo - N/A (CMint-only instruction, CMints don't support Pausable)
5. CTokenFreezeAccount - allowed when paused (freeze authority can still manage accounts)
6. CTokenThawAccount - allowed when paused (freeze authority can still manage accounts)
7. CloseTokenAccount - allowed when paused (account management, not token movement)

**Note:** CTokenMintTo and CTokenBurn only work with CMints (compressed mints). CMints do not support restricted extensions - only TokenMetadata is allowed. T22 mints with Pausable extension can only be used with CToken accounts via Transfer2 and CTokenTransfer.

---

### 5. DefaultAccountState

**Behavior:** When a mint has DefaultAccountState extension, new CToken accounts inherit the frozen state at creation time.

| Instruction | Validation Function | Check | Error |
|-------------|---------------------|-------|-------|
| CreateTokenAccount | `has_mint_extensions()` | Flags restricted extension, applies frozen state | `CompressionOnlyRequired` (6131) |
| Transfer2 (Decompress) | - | Restores frozen state from CompressedOnly extension | - |

**Validation paths:**
- `programs/compressed-token/program/src/extensions/check_mint_extensions.rs:213-220` - Detects `default_state_frozen` in `has_mint_extensions()`
- `programs/compressed-token/program/src/shared/initialize_ctoken_account.rs:190-198` - Applies frozen state in `initialize_ctoken_account()`

**Account Initialization:**
```rust
state: if mint_extensions.default_state_frozen {
    AccountState::Frozen as u8  // 2
} else {
    AccountState::Initialized as u8  // 1
}
```

**Frozen Account Behavior (pinocchio checks):**
- Transfer: Blocked (source or destination frozen)
- Approve: Blocked (source frozen)
- Revoke: Blocked (source frozen)
- Burn: Blocked (source frozen)
- Freeze/Thaw: Can override frozen state

**Unchecked instructions:**
1. CTokenMintTo - no frozen check
2. CTokenFreezeAccount - sets frozen state
3. CTokenThawAccount - clears frozen state
4. CloseTokenAccount - no frozen check

**Note:** Unlike other restricted extensions, DefaultAccountState does NOT have runtime validation in `check_mint_extensions()`. The frozen state is applied at account creation and checked by pinocchio during token operations.

---

## CompressedOnly Extension

The CompressedOnly extension preserves CToken account state during CompressAndClose operations, enabling full state restoration during Decompress.

### Data Structures

**State Extension** (`program-libs/ctoken-interface/src/state/extensions/compressed_only.rs`):
```rust
pub struct CompressedOnlyExtension {
    /// The delegated amount from the source CToken account's delegate field.
    pub delegated_amount: u64,
    /// Withheld transfer fee amount from the source CToken account.
    pub withheld_transfer_fee: u64,
    /// Whether the source was an ATA (1) or regular token account (0).
    pub is_ata: u8,
}
```

**Instruction Data** (`program-libs/ctoken-interface/src/instructions/extensions/compressed_only.rs`):
```rust
pub struct CompressedOnlyExtensionInstructionData {
    /// The delegated amount from the source CToken account's delegate field.
    pub delegated_amount: u64,
    /// Withheld transfer fee amount
    pub withheld_transfer_fee: u64,
    /// Whether the source CToken account was frozen when compressed.
    pub is_frozen: bool,
    /// Index of the compression operation that consumes this input.
    pub compression_index: u8,
    /// Whether the source CToken account was an ATA.
    pub is_ata: bool,
    /// ATA derivation bump (only used when is_ata=true).
    pub bump: u8,
    /// Index into packed_accounts for the actual owner (only used when is_ata=true).
    pub owner_index: u8,
}
```

### When Created (CompressAndClose)

**Path:** `programs/compressed-token/program/src/compressed_token/transfer2/compression/ctoken/compress_and_close.rs`

**Trigger:** `ZCompressionMode::CompressAndClose` with `compression_only=true` on source CToken account.

**Requirements:**
- Source CToken must have `compression_only` flag set
- Output compressed token must include CompressedOnly extension in TLV data
- Extension values must match source CToken state

**Validation (in `validate_compressed_token_account` and `validate_compressed_only_ext`):**
1. Owner must match (lines 103-115): output owner must match ctoken owner (or token account pubkey for ATA/compress_to_pubkey)
2. Amount must match (lines 117-121): compression_amount == output_amount == ctoken.amount
3. Mint must match (lines 123-129): output mint matches ctoken mint
4. Version must be ShaFlat (lines 131-134)
5. Extension required for compression_only or ATA accounts (lines 136-145)
6. Without extension: must not be frozen, must not have delegate (lines 147-156)
7. With extension (`validate_compressed_only_ext` function, lines 170-222):
   - 7a. `delegated_amount` must match (lines 177-181)
   - 7b. Delegate pubkey must match if present (lines 183-194)
   - 7c. `withheld_transfer_fee` must match (lines 196-209)
   - 7d. `is_frozen` must match (lines 211-214)
   - 7e. `is_ata` must match (lines 216-219)

**Source CToken Reset (lines 68-71 in `process_compress_and_close`):**
```rust
ctoken.base.amount.set(0);
// Unfreeze the account if frozen (frozen state is preserved in compressed token TLV)
// This allows the close_token_account validation to pass for frozen accounts
ctoken.base.set_initialized();
```

### When Consumed (Decompress)

**Path:** `programs/compressed-token/program/src/compressed_token/transfer2/compression/ctoken/decompress.rs`

**Trigger:** Decompressing a compressed token that has CompressedOnly extension.

**State Restoration (`validate_and_apply_compressed_only` function, lines 15-70):**
1. Return early if no decompress inputs or no CompressedOnly extension (lines 23-29)
2. Validate amount matches for ATA or compress_to_pubkey decompress (lines 31-46)
3. Validate destination ownership via `validate_destination` (lines 48-56)
4. Restore delegate pubkey and delegated_amount via `apply_delegate` (lines 58-59)
5. Restore `withheld_transfer_fee` via `apply_withheld_fee` (lines 61-62)
6. Restore frozen state via `ctoken.base.set_frozen()` (lines 64-67)

**Validation (`validate_destination`, lines 77-106):**
- For non-ATA: CToken owner must match input owner
- For ATA: destination address must match input owner (ATA pubkey), and CToken owner must match wallet owner

### State Preservation Matrix

| Field | Preserved (C&C) | Restored (Decompress) | Notes |
|-------|-----------------|----------------------|-------|
| delegated_amount | ✅ | ✅ | Stored in extension |
| withheld_transfer_fee | ✅ | ✅ | Restored to TransferFeeAccount |
| is_frozen | ✅ | ✅ | Restored via `set_frozen()` |
| is_ata | ✅ | ✅ | Used to validate ATA derivation on decompress |
| delegate pubkey | Validated | From input | Passed as instruction account |
| amount | ❌ (set to 0) | From compression | New amount from compressed token |
| close_authority | ❌ | ❌ | Not preserved |

### Error Codes

| Error | Code | Description |
|-------|------|-------------|
| `CompressAndCloseMissingCompressedOnlyExtension` | 6133 | Restricted mint CompressAndClose lacks CompressedOnly output |
| `CompressAndCloseDelegatedAmountMismatch` | 6135 | delegated_amount doesn't match source |
| `CompressAndCloseWithheldFeeMismatch` | 6137 | withheld_transfer_fee doesn't match source |
| `CompressAndCloseFrozenMismatch` | 6138 | is_frozen doesn't match source frozen state |
| `CompressAndCloseIsAtaMismatch` | N/A | is_ata doesn't match source ATA flag |
| `CompressAndCloseInvalidDelegate` | N/A | delegate pubkey doesn't match source |
| `CompressAndCloseDelegateNotAllowed` | N/A | delegate present but CompressedOnly extension missing |

---

## Validation Functions

### `assert_mint_extensions()`
**Path:** `programs/compressed-token/anchor/src/instructions/create_token_pool.rs:129-165`

**Used by:** CreateTokenPool (Anchor layer, pool creation time)

**Behavior:**
1. Deserialize mint with `PodStateWithExtensions<PodMint>::unpack()` (line 130-131)
2. Validate all extensions are in `ALLOWED_EXTENSION_TYPES` → `MintWithInvalidExtension` (lines 134-140)
3. If TransferFeeConfig exists: check fees are zero → `NonZeroTransferFeeNotSupported` (lines 142-153)
4. If TransferHook exists: check program_id is nil → `TransferHookNotSupported` (lines 155-162)

**Does NOT check:** Pausable state, PermanentDelegate (allowed at pool creation)

---

### `has_mint_extensions()`
**Path:** `programs/compressed-token/program/src/extensions/check_mint_extensions.rs:174-230`

**Used by:** CreateTokenAccount (detection only)

**Behavior:**
1. Return default flags if not Token-2022 mint (lines 176-179)
2. Deserialize mint with `PodStateWithExtensions<PodMint>::unpack()` (lines 181-184)
3. Get all extension types in a single call (line 187)
4. Validate all extensions are in `ALLOWED_EXTENSION_TYPES` → `MintWithInvalidExtension` (lines 196-200)
5. Detect which restricted extensions are present (lines 201-209)
6. Check if DefaultAccountState is set to Frozen (lines 213-220)
7. Return `MintExtensionFlags` with boolean flags

**Returns** (defined in `program-libs/ctoken-interface/src/token_2022_extensions.rs:59-75`):
```rust
MintExtensionFlags {
    has_pausable: bool,
    has_permanent_delegate: bool,
    has_default_account_state: bool,  // Extension exists (restricted)
    default_state_frozen: bool,       // Current state is Frozen (for CToken creation)
    has_transfer_fee: bool,
    has_transfer_hook: bool,
}
```

**Does NOT validate:** Extension values (fees, program_id, paused state). Only detects presence.

---

### `parse_mint_extensions()`
**Path:** `programs/compressed-token/program/src/extensions/check_mint_extensions.rs:53-117`

**Used by:** Internal helper for `check_mint_extensions()` and `build_mint_extension_cache()`

**Behavior:**
1. Return default if not Token-2022 mint (lines 56-59)
2. Deserialize mint with `PodStateWithExtensions<PodMint>::unpack()` (lines 61-64)
3. Compute `has_restricted_extensions` from extension types (lines 66-68)
4. Check if Pausable extension exists and paused state (lines 70-74)
5. Extract PermanentDelegate pubkey if exists (lines 76-84)
6. Check TransferFeeConfig for non-zero fees (lines 86-99)
7. Check TransferHook for non-nil program_id (lines 101-107)

**Returns** (defined in `check_mint_extensions.rs:22-40`):
```rust
MintExtensionChecks {
    permanent_delegate: Option<Pubkey>,  // For signer validation
    has_transfer_fee: bool,
    has_restricted_extensions: bool,     // For CompressAndClose validation
    is_paused: bool,                     // CompressAndClose bypasses this check
    has_non_zero_transfer_fee: bool,     // CompressAndClose bypasses this check
    has_non_nil_transfer_hook: bool,     // CompressAndClose bypasses this check
}
```

---

### `check_mint_extensions()`
**Path:** `programs/compressed-token/program/src/extensions/check_mint_extensions.rs:133-159`

**Used by:** Transfer2, CTokenTransfer (runtime validation)

**Parameters:**
- `mint_account: &AccountInfo` - The SPL Token 2022 mint
- `deny_restricted_extensions: bool` - If true, fails when mint has restricted extensions

**Behavior:** Wrapper around `parse_mint_extensions()` that throws errors for invalid states:
1. Call `parse_mint_extensions()` (line 138)
2. If `deny_restricted_extensions && has_restricted_extensions` → `MintHasRestrictedExtensions` (6142) (lines 140-145)
3. If `is_paused == true` → `MintPaused` (6127) (lines 147-150)
4. If `has_non_zero_transfer_fee` → `NonZeroTransferFeeNotSupported` (6129) (lines 151-153)
5. If `has_non_nil_transfer_hook` → `TransferHookNotSupported` (6130) (lines 154-156)

---

### `build_mint_extension_cache()`
**Path:** `programs/compressed-token/program/src/compressed_token/transfer2/check_extensions.rs:78-158`

**Used by:** Transfer2 (batch validation)

**Behavior:**
1. For each unique mint in inputs (lines 89-101):
   - If no outputs: call `parse_mint_extensions()` (bypass state checks for pure decompress)
   - Otherwise: call `check_mint_extensions()` with `deny_restricted_extensions`
   - Cache result in `ArrayMap<u8, MintExtensionChecks, 5>`
2. For each unique mint in compressions (lines 103-142):
   - CompressAndClose and full Decompress: use `parse_mint_extensions()` (bypass state checks)
   - Otherwise: use `check_mint_extensions()` with `deny_restricted_extensions`
3. Special handling for CompressAndClose mode (lines 121-140):
   - Mints with restricted extensions require CompressedOnly output extension
   - If missing → `CompressAndCloseMissingCompressedOnlyExtension` (6133)

**Returns:** `MintExtensionCache` (type alias defined at line 49) - Cached checks keyed by mint account index

---

## Error Codes

| Error | Code | Description |
|-------|------|-------------|
| `OwnerMismatch` | 6075 | Authority signature does not match owner/delegate |
| `MintPaused` | 6127 | Mint is paused |
| `NonZeroTransferFeeNotSupported` | 6129 | TransferFeeConfig has non-zero fees |
| `TransferHookNotSupported` | 6130 | TransferHook has non-nil program_id |
| `CompressionOnlyRequired` | 6131 | Restricted extension requires compression_only mode |
| `MintHasRestrictedExtensions` | 6142 | Cannot create compressed outputs with restricted extensions |


## Restricted Extension Enforcement for Compression

### Transfer2

**Enforcement:** `build_mint_extension_cache()` is called with `deny_restricted_extensions = !out_token_data.is_empty()`

**Flow:**
1. `build_mint_extension_cache()` computes `deny_restricted_extensions = !inputs.out_token_data.is_empty()` (line 86)
2. For input mints: calls `check_mint_extensions(mint, deny_restricted_extensions)` (line 97)
3. If `deny_restricted_extensions=true` and mint has restricted extensions → `MintHasRestrictedExtensions` (6142)

**Exception - CompressAndClose and Decompress modes:**
- CompressAndClose: calls `parse_mint_extensions()` to bypass state checks (line 112)
- Full Decompress (no outputs): calls `parse_mint_extensions()` to bypass state checks (lines 93-95)
- CompressAndClose still requires CompressedOnly output extension for restricted mints (lines 125-140)
- If missing → `CompressAndCloseMissingCompressedOnlyExtension` (6133)

**Path:** `programs/compressed-token/program/src/compressed_token/transfer2/processor.rs:61` calls `build_mint_extension_cache()`

### Anchor Instructions

**NOT ENFORCED** - The following anchor instructions do NOT check for restricted extensions:

1. `mint_to` - Can mint to compressed accounts from T22 mints with restricted extensions
2. `batch_compress` - Can compress SPL tokens from T22 mints with restricted extensions
3. `compress_spl_token_account` - Can compress SPL token account balance from T22 mints with restricted extensions
4. `transfer` (anchor) - Can compress/decompress with T22 mints with restricted extensions

**Gap:** These anchor instructions should either:
- Check for restricted extensions and fail with `MintHasRestrictedExtensions`
- Or be deprecated in favor of Transfer2 which properly enforces restrictions

## Open Questions

### 1. ~~Should DefaultAccountState be a restricted extension?~~ ✅ IMPLEMENTED

**Status:** Implemented. `DefaultAccountState` is now in `RESTRICTED_EXTENSION_TYPES`.

When a mint has the `DefaultAccountState` extension (regardless of current state), the `has_restricted_extensions()` flag is set to true via `has_default_account_state`, which enforces `compression_only` mode. This is necessary because:
1. The default state can be changed by mint authority at any time
2. Once compressed, we don't re-check the mint's DefaultAccountState when creating outputs
3. CToken accounts still respect the current frozen state for proper initialization

### 2. ~~How to enforce restricted extensions in anchor instructions?~~ ✅ IMPLEMENTED

**Status:** Implemented via different pool PDA derivation for restricted mints.

**Implementation:**
- `CreateTokenPool` uses `restricted_seed()` function (lines 21-39) to detect restricted extensions
- If mint has restricted extensions: `seeds = [b"pool", mint_pubkey, RESTRICTED_POOL_SEED]`
- Otherwise: `seeds = [b"pool", mint_pubkey]`
- `AddTokenPoolInstruction` follows same derivation pattern (lines 171-201)
- Anchor instructions use normal derivation → pool not found → CPI fails automatically

**Path:** `programs/compressed-token/anchor/src/instructions/create_token_pool.rs:17-39`

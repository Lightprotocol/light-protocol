# Token-2022 Extensions

This document describes how Token-2022 extensions are validated across compressed token instructions.

## Overview

The compressed token program supports 16 Token-2022 extension types. **5 restricted extensions** require instruction-level validation checks. Pure mint extensions (metadata, group, etc.) are allowed without explicit instruction support.

**Allowed extensions** (defined in `program-libs/ctoken-interface/src/token_2022_extensions.rs:23-43`):

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

| Instruction          | TransferFee       | DefaultState       | PermanentDelegate  | TransferHook      | Pausable           |
|----------------------|-------------------|--------------------|--------------------|-------------------|--------------------|
| CreateTokenAccount   | requires comp_only| applies frozen     | requires comp_only | requires comp_only| requires comp_only |
| Transfer2 (compress) | blocked           | -                  | blocked            | blocked           | blocked if paused  |
| Transfer2 (c→c)      | blocked           | -                  | blocked            | blocked           | blocked            |
| Transfer2 (decompress)| allowed          | restores frozen    | allowed            | allowed           | allowed            |
| Transfer2 (C&C)      | allowed           | preserved          | allowed            | allowed           | allowed            |
| CTokenTransfer       | fees must be 0    | frozen blocked     | authority check    | hook must be nil  | blocked if paused  |
| CTokenApprove        | -                 | frozen blocked     | -                  | -                 | -                  |
| CTokenRevoke         | -                 | frozen blocked     | -                  | -                 | -                  |
| CTokenBurn           | N/A (CMint-only)  | frozen blocked     | N/A (CMint-only)   | N/A (CMint-only)  | N/A (CMint-only)   |
| CTokenMintTo         | N/A (CMint-only)  | -                  | N/A (CMint-only)   | N/A (CMint-only)  | N/A (CMint-only)   |
| CTokenFreeze/Thaw    | -                 | -                  | -                  | -                 | -                  |
| CloseTokenAccount    | -                 | -                  | -                  | -                 | -                  |
| CreateTokenPool      | fees must be 0    | -                  | -                  | hook must be nil  | -                  |

**Key:**
- `requires comp_only` = Extension triggers compression_only requirement
- `blocked` = Operation fails with MintHasRestrictedExtensions (6121)
- `bypassed` = CompressAndClose skips all extension validation
- `fees must be 0` / `hook must be nil` = Specific validation check
- `frozen blocked` = Account frozen state prevents operation (pinocchio check)
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
| CreateTokenAccount | `has_mint_extensions()` | Flags restricted extension | `CompressionOnlyRequired` (6097) |

**Validation paths:**
- `programs/compressed-token/anchor/src/instructions/create_token_pool.rs:119-130`
- `programs/compressed-token/program/src/extensions/check_mint_extensions.rs:85-101`

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
| CreateTokenAccount | `has_mint_extensions()` | Flags restricted extension | `CompressionOnlyRequired` (6097) |

**Validation paths:**
- `programs/compressed-token/anchor/src/instructions/create_token_pool.rs:132-139`
- `programs/compressed-token/program/src/extensions/check_mint_extensions.rs:103-108`

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
| CreateTokenAccount | `has_mint_extensions()` | Flags restricted extension | `CompressionOnlyRequired` (6097) |
| Transfer2 | `check_mint_extensions()` → `verify_owner_or_delegate_signer()` | Extract delegate pubkey, then validate authority is owner OR delegate. If authority matches permanent delegate, that account must be a signer. | `OwnerMismatch` (6077) or `MissingRequiredSignature` |
| CTokenTransfer | `check_mint_extensions()` → `verify_owner_or_delegate_signer()` | Extract delegate pubkey, then validate authority is owner OR delegate. If authority matches permanent delegate, that account must be a signer. | `OwnerMismatch` (6077) or `MissingRequiredSignature` |

**Validation paths:**
- `programs/compressed-token/program/src/extensions/check_mint_extensions.rs:76-83` - Extracts delegate pubkey
- `programs/compressed-token/program/src/shared/owner_validation.rs:48-55` - Validates delegate signer
- `programs/compressed-token/program/src/transfer/shared.rs:164-179` - `validate_permanent_delegate()`

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
| CreateTokenAccount | `has_mint_extensions()` | Flags restricted extension | `CompressionOnlyRequired` (6097) |
| Transfer2 | `check_mint_extensions()` | `pausable_config.paused == false` | `MintPaused` (6131) |
| CTokenTransfer | `check_mint_extensions()` | `pausable_config.paused == false` | `MintPaused` (6131) |

**Validation path:**
- `programs/compressed-token/program/src/extensions/check_mint_extensions.rs:69-73`

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
| CreateTokenAccount | `has_mint_extensions()` | Flags restricted extension, applies frozen state | `CompressionOnlyRequired` (6097) |
| Transfer2 (Decompress) | - | Restores frozen state from CompressedOnly extension | - |

**Validation paths:**
- `programs/compressed-token/program/src/extensions/check_mint_extensions.rs:209-218` - Detects `default_state_frozen`
- `programs/compressed-token/program/src/shared/initialize_ctoken_account.rs:96-100` - Applies frozen state

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
    pub delegated_amount: u64,
    pub withheld_transfer_fee: u64,
}
```

**Instruction Data** (`program-libs/ctoken-interface/src/instructions/extensions/compressed_only.rs`):
```rust
pub struct CompressedOnlyExtensionInstructionData {
    pub delegated_amount: u64,
    pub withheld_transfer_fee: u64,
    pub is_frozen: bool,
    pub compression_index: u8,
}
```

### When Created (CompressAndClose)

**Path:** `programs/compressed-token/program/src/transfer2/compression/ctoken/compress_and_close.rs`

**Trigger:** `ZCompressionMode::CompressAndClose` with `compression_only=true` on source CToken account.

**Requirements:**
- Source CToken must have `compression_only` flag set
- Output compressed token must include CompressedOnly extension in TLV data
- Extension values must match source CToken state

**Validation (lines 168-261):**
1. If source has `compression_only=true`, CompressedOnly extension is required
2. `delegated_amount` must match source CToken's `delegated_amount`
3. `withheld_transfer_fee` must match source's TransferFeeAccount withheld amount
4. `is_frozen` must match source CToken's frozen state (`state == 2`)
5. If source is frozen but extension missing → `CompressAndCloseMissingCompressedOnlyExtension`

**Source CToken Reset:**
```rust
ctoken.base.amount.set(0);
ctoken.base.set_initialized();  // Unfreeze before closing
```

### When Consumed (Decompress)

**Path:** `programs/compressed-token/program/src/transfer2/compression/ctoken/decompress.rs`

**Trigger:** Decompressing a compressed token that has CompressedOnly extension.

**State Restoration (lines 66-125):**
1. Extract CompressedOnly data from input TLV
2. Restore delegate pubkey (from instruction input account)
3. Restore `delegated_amount` to destination CToken
4. Restore `withheld_transfer_fee` to TransferFeeAccount extension
5. Restore frozen state via `ctoken.base.set_frozen()`

**Validation:**
- Destination CToken must be fresh (amount=0, no delegate, no delegated_amount)
- Destination owner must match

### State Preservation Matrix

| Field | Preserved (C&C) | Restored (Decompress) | Notes |
|-------|-----------------|----------------------|-------|
| delegated_amount | ✅ | ✅ | Stored in extension |
| withheld_transfer_fee | ✅ | ✅ | Restored to TransferFeeAccount |
| is_frozen | ✅ | ✅ | Restored via `set_frozen()` |
| delegate pubkey | Validated | From input | Passed as instruction account |
| amount | ❌ (set to 0) | From compression | New amount from compressed token |
| close_authority | ❌ | ❌ | Not preserved |

### Error Codes

| Error | Code | Description |
|-------|------|-------------|
| `CompressAndCloseMissingCompressedOnlyExtension` | 6122 | Restricted mint CompressAndClose lacks CompressedOnly output |
| `CompressAndCloseDelegatedAmountMismatch` | 6123 | delegated_amount doesn't match source |
| `CompressAndCloseWithheldTransferFeeMismatch` | 6124 | withheld_transfer_fee doesn't match source |
| `CompressAndCloseFrozenMismatch` | 6125 | is_frozen doesn't match source frozen state |

---

## Validation Functions

### `assert_mint_extensions()`
**Path:** `programs/compressed-token/anchor/src/instructions/create_token_pool.rs:106-142`

**Used by:** CreateTokenPool (Anchor layer, pool creation time)

**Behavior:**
1. Deserialize mint with `PodStateWithExtensions<PodMint>::unpack()`
2. Validate all extensions are in `ALLOWED_EXTENSION_TYPES` → `MintWithInvalidExtension`
3. If TransferFeeConfig exists: check fees are zero → `NonZeroTransferFeeNotSupported`
4. If TransferHook exists: check program_id is nil → `TransferHookNotSupported`

**Does NOT check:** Pausable state, PermanentDelegate (allowed at pool creation)

---

### `has_mint_extensions()`
**Path:** `programs/compressed-token/program/src/extensions/check_mint_extensions.rs:130-184`

**Used by:** CreateTokenAccount (detection only)

**Behavior:**
1. Return default flags if not Token-2022 mint
2. Deserialize mint with `PodStateWithExtensions<PodMint>::unpack()`
3. Validate all extensions are in `ALLOWED_EXTENSION_TYPES` → `MintWithInvalidExtension`
4. Return `MintExtensionFlags` with boolean flags

**Returns:**
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

### `check_mint_extensions()`
**Path:** `programs/compressed-token/program/src/extensions/check_mint_extensions.rs:43-115`

**Used by:** Transfer2, CTokenTransfer (runtime validation)

**Parameters:**
- `mint_account: &AccountInfo` - The SPL Token 2022 mint
- `deny_restricted_extensions: bool` - If true, fails when mint has restricted extensions

**Behavior:**
1. Return default if not Token-2022 mint
2. Deserialize mint with `PodStateWithExtensions<PodMint>::unpack()`
3. Compute `has_restricted_extensions` from extension types
4. If `deny_restricted_extensions && has_restricted_extensions` → `MintHasRestrictedExtensions`
5. If Pausable exists and `paused == true` → `MintPaused`
6. Extract PermanentDelegate pubkey if exists (for downstream signer validation)
7. If TransferFeeConfig exists: check fees are zero → `NonZeroTransferFeeNotSupported`
8. If TransferHook exists: check program_id is nil → `TransferHookNotSupported`

**Returns:**
```rust
MintExtensionChecks {
    permanent_delegate: Option<Pubkey>,  // For signer validation
    has_transfer_fee: bool,
    has_restricted_extensions: bool,     // For CompressAndClose validation
}
```

---

### `build_mint_extension_cache()`
**Path:** `programs/compressed-token/program/src/transfer2/check_extensions.rs:65-142`

**Used by:** Transfer2 (batch validation)

**Behavior:**
1. For each unique mint in inputs and compressions (max 5 mints):
   - Call `check_mint_extensions()` with appropriate `deny_restricted_extensions`
   - Cache result in `ArrayMap<u8, MintExtensionChecks, 5>`
2. Special handling for CompressAndClose mode:
   - Mints with restricted extensions require CompressedOnly output extension
   - If missing → `CompressAndCloseMissingCompressedOnlyExtension`

**Returns:** `MintExtensionCache` - Cached checks keyed by mint account index

---

## Error Codes

| Error | Code | Description |
|-------|------|-------------|
| `NonZeroTransferFeeNotSupported` | 6129 | TransferFeeConfig has non-zero fees |
| `TransferHookNotSupported` | 6130 | TransferHook has non-nil program_id |
| `MintPaused` | 6131 | Mint is paused |
| `CompressionOnlyRequired` | 6097 | Restricted extension requires compression_only mode |
| `MintHasRestrictedExtensions` | 6121 | Cannot create compressed outputs with restricted extensions |
| `OwnerMismatch` | 6077 | Authority signature does not match owner/delegate |


## Restricted Extension Enforcement for Compression

### Transfer2

**Enforcement:** `build_mint_extension_cache()` is called with `deny_restricted_extensions = !no_output_compressed_accounts`

**Flow:**
1. `Transfer2Config::from_instruction_data()` computes `no_output_compressed_accounts = out_token_data.is_empty()`
2. `build_mint_extension_cache()` calls `check_mint_extensions(mint, deny_restricted_extensions)`
3. If `deny_restricted_extensions=true` and mint has restricted extensions → `MintHasRestrictedExtensions` (6121)

**Exception - CompressAndClose mode:**
- Always passes `deny_restricted_extensions=false` to `check_mint_extensions()`
- Instead requires CompressedOnly output extension
- If missing → `CompressAndCloseMissingCompressedOnlyExtension`

**Path:** `programs/compressed-token/program/src/transfer2/processor.rs:61-65`

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

### 2. How to enforce restricted extensions in anchor instructions?

**Different pool PDA derivation for restricted mints**
- Current: `seeds = [b"pool", mint_pubkey]` for all mints
- Proposed: `seeds = [b"pool", mint_pubkey, b"restricted"]` for restricted mints
- `CreateTokenPool` detects restricted extensions → creates pool at different PDA
- Anchor instructions use normal derivation → pool not found → CPI fails automatically
- Transfer2 derives correct pool based on mint extension flags from cache
- Pros: No changes to anchor instruction code, implicit enforcement
- Cons: SDK/client changes needed, Transfer2 pool derivation update required

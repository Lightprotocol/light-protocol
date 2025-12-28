# T22 vs CToken: Restricted Extensions Comparison

This document compares the behavior of 5 restricted Token-2022 extensions between SPL Token-2022 (T22) and the CToken implementation.

**Reference Documents:**
- T22 behavior: `RESTRICTED_T22_EXTENSIONS.md`
- CToken behavior: `EXTENSIONS.md`

---

## Quick Reference

| Aspect                    | T22                          | CToken                              |
|---------------------------|------------------------------|-------------------------------------|
| TransferFee handling      | Fees deducted & withheld     | Fees must be 0 (blocked otherwise)  |
| TransferHook execution    | CPI invoked on transfer      | program_id must be nil (no CPI)     |
| PermanentDelegate scope   | Transfer + Burn              | Transfer + Burn (same)              |
| Pausable: MintTo/Burn     | Blocked when paused          | N/A (CMint-only, no extensions)     |
| Account extensions        | Per-extension markers        | All restricted add markers          |
| Compression bypass        | N/A                          | CompressAndClose/FullDecompress bypass |

---

## 1. TransferFeeConfig

### Shared Behavior

- Both read TransferFeeConfig extension from mint
- Both check `older_transfer_fee` and `newer_transfer_fee` fields

### Key Differences

| Aspect            | T22                                              | CToken                                           |
|-------------------|--------------------------------------------------|--------------------------------------------------|
| Fee handling      | Deducted from transfer, withheld in destination  | Must be 0, otherwise `NonZeroTransferFeeNotSupported` |
| CloseAccount      | Blocked if `withheld_amount > 0`                 | No withheld check (fees always 0)                |
| Account extension | TransferFeeAmount with `withheld_amount` field   | TransferFeeAccount marker (no withheld tracking) |

### T22 Features Not Implemented

1. `HarvestWithheldTokensToMint` - Move withheld fees from accounts to mint
2. `WithdrawWithheldTokensFromMint` - Withdraw accumulated fees to authority
3. `SetTransferFee` - Update fee configuration (2-epoch delay)
4. `TransferCheckedWithFee` - Transfer with fee parameter validation

### Design Rationale

CToken requires zero fees because compressed tokens cannot track withheld amounts per-account in compressed state. The CompressedOnlyExtension preserves `withheld_transfer_fee` for tokens that had fees before compression, but no new fees can accrue.

---

## 2. DefaultAccountState

### Shared Behavior

- Both apply frozen state at account initialization
- Both allow Freeze/Thaw to override state
- Frozen accounts block: Transfer, Approve, Revoke, Burn

### Key Differences

| Aspect            | T22                                    | CToken                                |
|-------------------|----------------------------------------|---------------------------------------|
| Account extension | None (state stored in base Account)    | No marker added                       |
| Update capability | `UpdateDefaultAccountState` instruction | No update (reads mint state directly) |
| MintTo to frozen  | Blocked                                | Blocked (pinocchio check)             |

### T22 Features Not Implemented

1. `InitializeDefaultAccountState` - Initialize extension on mint
2. `UpdateDefaultAccountState` - Change default state for future accounts

### Design Rationale

CToken reads the DefaultAccountState from the T22 mint directly at account creation time. Mint-level instructions (Initialize/Update) are executed on the T22 mint, not through CToken.

---

## 3. PermanentDelegate

### Shared Behavior

- Permanent delegate can authorize transfers (same authority hierarchy: permanent delegate > regular delegate > owner)
- Permanent delegate can authorize burns
- Approve/Revoke: owner only (permanent delegate has no special privileges)
- Transfers/burns by permanent delegate do not consume `delegated_amount`

### Key Differences

| Aspect            | T22                                | CToken                                  |
|-------------------|------------------------------------|-----------------------------------------|
| Account extension | None                               | PermanentDelegateAccountExtension marker |
| SetAuthority      | Delegate can renounce authority    | Not implemented (T22 mint instruction)  |

### T22 Features Not Implemented

1. `SetAuthority(PermanentDelegate)` - Transfer or renounce permanent delegate authority

### Design Rationale

CToken adds an account marker to identify accounts belonging to mints with permanent delegate. This enables `compression_only` enforcement - accounts must be explicitly created in compression_only mode to ensure state is preserved during CompressAndClose.

---

## 4. TransferHook

### Shared Behavior

- Both check for TransferHook extension on mint
- Both add marker extension to accounts (though with different contents)

### Key Differences

| Aspect            | T22                                           | CToken                                       |
|-------------------|-----------------------------------------------|----------------------------------------------|
| Hook execution    | CPI to program_id after balance update        | No CPI (program_id must be nil)              |
| Reentrancy guard  | `transferring` flag in TransferHookAccount    | No guard needed (no CPI)                     |
| Account extension | TransferHookAccount with `transferring` field | TransferHookAccount marker (no transferring) |

### T22 Features Not Implemented

1. `spl_transfer_hook_interface::onchain::invoke_execute()` - Hook CPI execution
2. `Update` - Change hook program_id after initialization

### Design Rationale

Transfer hooks invoke external programs that cannot access compressed state. Since compressed tokens aren't visible to external programs, hooks cannot validate or act on compressed transfers. CToken requires `program_id = nil` to ensure hooks are disabled before compression.

---

## 5. Pausable

### Shared Behavior

- Both read `paused` state from PausableConfig extension
- Transfers blocked when paused (CTokenTransfer, Transfer2 compress)
- Approve/Revoke/Freeze/Thaw allowed when paused

### Key Differences

| Aspect                   | T22                       | CToken                            |
|--------------------------|---------------------------|-----------------------------------|
| MintTo when paused       | Blocked (`MintPaused`)    | N/A (CTokenMintTo is CMint-only)  |
| Burn when paused         | Blocked (`MintPaused`)    | N/A (CTokenBurn is CMint-only)    |
| Pause/Resume             | Direct instructions       | Not implemented (T22 mint instr)  |
| Full Decompress (paused) | N/A                       | ALLOWED (bypasses check)          |
| CompressAndClose         | N/A                       | ALLOWED (bypasses check)          |

### T22 Features Not Implemented

1. `Pause` - Set `paused = true` on mint
2. `Resume` - Set `paused = false` on mint

### Design Rationale

**CTokenMintTo/CTokenBurn - CMint only:**
CTokenMintTo and CTokenBurn instructions only work with CMints (compressed mints). CMints do not support restricted extensions - only TokenMetadata is allowed. Therefore, pausable checks are not applicable to these instructions. T22 mints with Pausable extension can only be used with CToken accounts via Transfer2 (compress/decompress).

**Full Decompress/CompressAndClose bypass:**
Users who compressed tokens before a pause should be able to recover them. CompressAndClose allows foresters to reclaim rent even when paused. These operations use `parse_mint_extensions()` (extract data only) instead of `check_mint_extensions()` (validate state).

**Note:** "Full decompress" means decompress operations with no compressed outputs (`inputs.out_token_data.is_empty()`). Decompress operations that also create new compressed outputs are subject to normal validation.

---

## 6. Cross-Cutting Differences

### CMint vs T22 Mint Limitations

**CMints (Compressed Mints):**
- Only support TokenMetadata extension
- No restricted extensions (Pausable, TransferFee, TransferHook, PermanentDelegate, DefaultAccountState)
- Used by: CTokenMintTo, CTokenBurn

**T22 Mints with Restricted Extensions:**
- Supported only via CToken accounts (not CMints)
- CToken accounts for restricted mints require `compression_only` mode
- Used by: Transfer2 (compress/decompress), CTokenTransfer, CTokenApprove, CTokenRevoke, etc.

**Implication:** CTokenMintTo and CTokenBurn do not need pausable/extension checks because they only operate on CMints which cannot have those extensions.

### compression_only Mode (CToken-specific)

Required when mint has any restricted extension:
- Enforced at CreateTokenAccount via `has_mint_extensions()`
- Prevents creation of regular compressed token outputs for restricted mints
- Error: `CompressionOnlyRequired` (6131)

Enables:
- State preservation during CompressAndClose (delegated_amount, withheld_transfer_fee, frozen state)
- Safe round-trip compression/decompression without losing account state

### CompressAndClose/Decompress Bypass (CToken-specific)

```rust
// Path: src/transfer2/check_extensions.rs:106-114
let is_full_decompress =
    compression.mode.is_decompress() && inputs.out_token_data.is_empty();
let checks = if compression.mode.is_compress_and_close() || is_full_decompress {
    // CompressAndClose and Decompress bypass extension state checks
    parse_mint_extensions(mint_account)?  // Extract data only
} else {
    check_mint_extensions(mint_account, deny_restricted_extensions)?  // Validate state
};
```

**Note:** Only "full decompress" (decompress without creating new compressed outputs) bypasses
state checks. Decompress operations that create additional compressed outputs are subject to
normal validation via `check_mint_extensions`.

This allows:
- **Full Decompress when paused:** Users can recover tokens compressed before pause (when no compressed outputs)
- **CompressAndClose when paused:** Foresters can reclaim rent exemption
- **Operations after fee/hook changes:** Users aren't locked out by mint config changes

### Account Extension Markers

| Extension           | T22 Adds Marker     | CToken Adds Marker               |
|---------------------|---------------------|----------------------------------|
| TransferFeeConfig   | TransferFeeAmount   | TransferFeeAccount               |
| DefaultAccountState | None                | None                             |
| PermanentDelegate   | None                | PermanentDelegateAccountExtension |
| TransferHook        | TransferHookAccount | TransferHookAccount              |
| Pausable            | PausableAccount     | PausableAccount                  |

**Key difference:** T22's TransferFeeAmount and TransferHookAccount have data fields. CToken uses zero-sized markers.

### Validation Function Comparison

| Validation Point | T22 | CToken |
|------------------|-----|--------|
| Account creation | Extension-specific initialization | `has_mint_extensions()` - flags restricted extensions |
| Transfer | Extension-specific processors | `check_mint_extensions()` - validates all extension state |
| Pool creation | N/A | `assert_mint_extensions()` - fees=0, hook=nil |

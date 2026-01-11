# Account Checks Security Documentation

This document provides comprehensive documentation of all Solana account validations in the compressed-token program. For each instruction, it details every account accessed, how it is validated, and what security checks are applied.

## Table of Contents

1. [Overview](#1-overview)
2. [Validation Framework Reference](#2-validation-framework-reference)
3. [Instructions by Category](#3-instructions-by-category)
   - [3.1 Account Management](#31-account-management)
   - [3.2 CToken Operations](#32-ctoken-operations)
   - [3.3 Compressed Token Operations](#33-compressed-token-operations)
   - [3.4 Compressible Rent Management](#34-compressible-rent-management)
   - [3.5 Token Pool Operations (Anchor)](#35-token-pool-operations-anchor)
4. [Potential Gaps and Recommendations](#4-potential-gaps-and-recommendations)
5. [Quick Reference Tables](#5-quick-reference-tables)

---

## 1. Overview

### 1.1 Purpose and Scope

This document covers account validation for **29 instructions** in the compressed-token program:
- **18 Pinocchio-based instructions** - Native Solana instructions with manual account parsing
- **11 Anchor-based instructions** - Instructions using Anchor framework constraints

### 1.2 Validation Mechanisms Used

| Mechanism | Description | Location |
|-----------|-------------|----------|
| `AccountIterator` | Sequential account parsing with named error locations | `program-libs/account-checks/src/account_iterator.rs` |
| `ProgramPackedAccounts` | Index-based dynamic account access | `program-libs/account-checks/src/packed_accounts.rs` |
| `check_signer()` | Verify account is transaction signer | `program-libs/account-checks/src/checks.rs:121` |
| `check_mut()` | Verify account is writable | `program-libs/account-checks/src/checks.rs:128` |
| `check_non_mut()` | Verify account is read-only | `program-libs/account-checks/src/checks.rs:43` |
| `check_owner()` | Verify account program ownership | `program-libs/account-checks/src/checks.rs:135` |
| `check_discriminator()` | Verify 8-byte account type prefix | `program-libs/account-checks/src/checks.rs:78` |
| `check_pda_seeds()` | Verify PDA derivation with find_program_address | `program-libs/account-checks/src/checks.rs:158` |
| `check_pda_seeds_with_bump()` | Verify PDA derivation with known bump | `program-libs/account-checks/src/checks.rs:170` |
| `verify_owner_or_delegate_signer()` | Token authority validation (owner/delegate/permanent_delegate) | `src/shared/owner_validation.rs:30` |
| `check_ctoken_owner()` | Compression authority validation | `src/shared/owner_validation.rs:83` |

### 1.3 Error Code Ranges

| Range | Source | Description |
|-------|--------|-------------|
| 20000-20015 | `AccountError` | Account validation errors from account-checks |
| 18001-18066 | `CTokenError` | Compressed token specific errors |
| 6000+ | `ErrorCode` | Anchor compressed token errors |

**AccountError Codes (20000-20015):**

| Code | Error | Description |
|------|-------|-------------|
| 20000 | `InvalidDiscriminator` | Account type prefix mismatch |
| 20001 | `AccountOwnedByWrongProgram` | Owner check failed |
| 20002 | `AccountNotMutable` | Mutability check failed |
| 20003 | `BorrowAccountDataFailed` | Cannot borrow account data |
| 20004 | `InvalidAccountSize` | Account size mismatch |
| 20005 | `AccountMutable` | Non-mutability check failed |
| 20006 | `AlreadyInitialized` | Account discriminator not zeroed |
| 20007 | `InvalidAccountBalance` | Insufficient lamports for rent |
| 20008 | `FailedBorrowRentSysvar` | Cannot read rent sysvar |
| 20009 | `InvalidSigner` | Signer check failed |
| 20010 | `InvalidSeeds` | PDA derivation mismatch |
| 20011 | `InvalidProgramId` | Program ID check failed |
| 20012 | `ProgramNotExecutable` | Program not executable |
| 20013 | `AccountNotZeroed` | Account not zeroed for init |
| 20014 | `NotEnoughAccountKeys` | Insufficient accounts provided |
| 20015 | `InvalidAccount` | Generic account validation failure |

---

## 2. Validation Framework Reference

### 2.1 AccountIterator Pattern

Sequential account parsing with automatic validation and error location tracking.

```rust
let mut iter = AccountIterator::new(account_infos);
let fee_payer = iter.next_signer_mut("fee_payer")?;  // Checks: signer + mutable
let mint = iter.next_non_mut("mint")?;                // Checks: non-mutable
let authority = iter.next_signer("authority")?;       // Checks: signer only
let optional = iter.next_option_mut("opt", condition)?; // Conditional mutable
```

**Methods and Their Checks:**

| Method | Signer | Mutable | Non-Mutable |
|--------|--------|---------|-------------|
| `next_account()` | - | - | - |
| `next_signer()` | Y | - | - |
| `next_mut()` | - | Y | - |
| `next_non_mut()` | - | - | Y |
| `next_signer_mut()` | Y | Y | - |
| `next_signer_non_mut()` | Y | - | Y |
| `next_option()` | - | - | - |
| `next_option_mut()` | - | Y | - |
| `next_option_signer()` | Y | - | - |

### 2.2 Authority Validation Functions

**`verify_owner_or_delegate_signer()`** - Token operations authorization:
```
Location: src/shared/owner_validation.rs:30-78
Accepts:
1. Owner account is signer -> OK
2. Delegate account is signer -> OK
3. Permanent delegate (from mint extension) is signer -> OK
Error: ErrorCode::OwnerMismatch (if none are signers)
```

**`check_ctoken_owner()`** - Compression operations authorization:
```
Location: src/shared/owner_validation.rs:83-113
Checks:
1. Authority account must be signer -> InvalidSigner
2. Authority matches owner -> OK
3. Authority matches permanent delegate -> OK
Error: ErrorCode::OwnerMismatch (if neither matches)
```

**`check_token_program_owner()`** - Token program ownership:
```
Location: src/shared/owner_validation.rs:14-25
Accepts: SPL Token | Token-2022 | CToken program
Error: ProgramError::IncorrectProgramId
```

---

## 3. Instructions by Category

### 3.1 Account Management

#### 3.1.1 CreateTokenAccount (Discriminator: 18)

**Source:** `src/ctoken/create.rs:21-108`
**Enum:** `InstructionType::CreateTokenAccount`

##### Account Layout

| Index | Account | Mut | Signer | Checks | Error |
|-------|---------|-----|--------|--------|-------|
| 0 | token_account | Y | Conditional* | `next_signer_mut()` or `next_mut()` | 20002/20009 |
| 1 | mint | N | N | `next_non_mut()` | 20005 |
| 2** | payer | Y | Y | `next_signer_mut()` | 20002/20009 |
| 3** | config_account | N | N | `next_config_account()` | 20001/20000 |
| 4** | system_program | N | N | `next_non_mut()` | 20005 |
| 5** | rent_payer | Y | N | `next_mut()` | 20002 |

*Conditional: Signer required only when `compressible_config` is Some
**Accounts 2-5 only required when `compressible_config` is Some

##### Account Details

**[0] token_account**
- **Mutability:** Always mutable
- **Signer:** Required for compressible accounts (PDA signer), not required for non-compressible
- **Owner:** For non-compressible, must already be owned by CToken program (implicit via write)
- **Validation Code:**
  ```rust
  // src/ctoken/create.rs:46-50
  let token_account = if is_compressible {
      iter.next_signer_mut("token_account")?
  } else {
      iter.next_mut("token_account")?
  };
  ```

**[1] mint**
- **Mutability:** Read-only
- **Owner:** SPL Token or Token-2022 (checked during extension parsing)
- **Validation:** Extensions parsed via `has_mint_extensions(mint)`

**[3] config_account (if compressible)**
- **Owner:** Registry program `Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX`
- **Discriminator:** `CompressibleConfig::LIGHT_DISCRIMINATOR`
- **State:** Must be ACTIVE
- **Validation Code:**
  ```rust
  // src/shared/config_account.rs
  check_owner(&registry_program_id, config_account)?;
  check_discriminator::<CompressibleConfig>(data)?;
  config.validate_active()?;
  ```

##### Security Analysis

| Attack Pattern | Protected | Check Location | Notes |
|----------------|-----------|----------------|-------|
| Owner before read | Y | config via `check_owner()` | Mint extensions parsed after owner implicit via program |
| Discriminator | Y | `check_discriminator::<CompressibleConfig>()` | Config account type verified |
| Signer check | Y | `next_signer_mut()` for compressible | Token account signer for compressible path |
| PDA verification | Y | `check_seeds()` on compress_to_pubkey | If provided, verified against token_account |
| Frontrunning | Partial | Token account address deterministic if compressible | Non-compressible can be frontrun |

---

#### 3.1.2 CreateAssociatedCTokenAccount (Discriminator: 100)

**Source:** `src/ctoken/create_ata.rs:20-142`
**Enum:** `InstructionType::CreateAssociatedCTokenAccount`

##### Account Layout

| Index | Account | Mut | Signer | Checks | Error |
|-------|---------|-----|--------|--------|-------|
| 0 | owner | N | N | `next_account()` | - |
| 1 | mint | N | N | `next_account()` | - |
| 2 | fee_payer | Y | Y | `next_signer_mut()` | 20002/20009 |
| 3 | associated_token_account | Y | N | `next_mut()` | 20002 |
| 4 | system_program | N | N | `next_non_mut()` | 20005 |
| 5* | compressible_config | N | N | `next_config_account()` | 20001/20000 |
| 6* | rent_payer | Y | N | `next_mut()` | 20002 |

*Accounts 5-6 only required when `compressible_config` is Some

##### Account Details

**[3] associated_token_account**
- **Owner Check:** Must be System program (uninitialized) at `create_ata.rs:75-77`
  ```rust
  if !associated_token_account.is_owned_by(&[0u8; 32]) {
      return Err(ProgramError::IllegalOwner);
  }
  ```
- **PDA Derivation:** Verified via seeds `[owner, ctoken_program, mint, bump]`

##### Security Analysis

| Attack Pattern | Protected | Check Location | Notes |
|----------------|-----------|----------------|-------|
| Owner before read | Y | System program check | Must be uninitialized |
| PDA verification | Y | ATA seeds derivation | Address deterministic |
| Frontrunning | Y | PDA + owner check | Cannot frontrun with different settings |
| Account revival | N/A | New account creation | Not applicable |

---

#### 3.1.3 CreateAssociatedTokenAccountIdempotent (Discriminator: 102)

**Source:** `src/ctoken/create_ata.rs:29-34`
**Enum:** `InstructionType::CreateAssociatedTokenAccountIdempotent`

Same as CreateAssociatedCTokenAccount with idempotent mode enabled.

**Additional Check for Idempotent Mode:**
```rust
// src/ctoken/create_ata.rs:67-72
if IDEMPOTENT {
    validate_ata_derivation(associated_token_account, owner_bytes, mint_bytes, bump)?;
    if associated_token_account.is_owned_by(&crate::LIGHT_CPI_SIGNER.program_id) {
        return Ok(()); // Already exists, return early
    }
}
```

---

#### 3.1.4 CloseTokenAccount (Discriminator: 9)

**Source:** `src/ctoken/close/processor.rs:17-30`
**Accounts:** `src/ctoken/close/accounts.rs:8-33`
**Enum:** `InstructionType::CloseTokenAccount`

##### Account Layout

| Index | Account | Mut | Signer | Checks | Error |
|-------|---------|-----|--------|--------|-------|
| 0 | token_account | Y | N | `next_mut()` + `check_owner()` | 20001/20002 |
| 1 | destination | Y | N | `next_mut()` | 20002 |
| 2 | authority | N | Y | `next_signer()` | 20009 |
| 3* | rent_sponsor | Y | N | `next_mut()` | 20002 |

*rent_sponsor required only if token_account has Compressible extension

##### Account Details

**[0] token_account**
- **Owner:** CToken program
- **Discriminator:** Implicitly checked via `CToken::from_account_info_mut_checked()`
- **Validation Code:**
  ```rust
  // src/ctoken/close/accounts.rs:20-21
  let token_account = iter.next_mut("token_account")?;
  check_owner(&LIGHT_CPI_SIGNER.program_id, token_account)?;
  ```

**[2] authority**
- **Authorization:** Must match close_authority (if set) OR owner
- **Signer Check:** `check_signer(accounts.authority)` at `close/processor.rs:111`
- **Validation Code:**
  ```rust
  // src/ctoken/close/processor.rs:78-98
  if let Some(close_authority) = ctoken.close_authority() {
      if !pubkey_eq(ctoken.close_authority.array_ref(), accounts.authority.key()) {
          return Err(ErrorCode::OwnerMismatch.into());
      }
  } else {
      if !pubkey_eq(ctoken.owner.array_ref(), accounts.authority.key()) {
          return Err(ErrorCode::OwnerMismatch.into());
      }
  }
  ```

**[3] rent_sponsor (if compressible)**
- **Validation:** Must match `compression.info.rent_sponsor` stored in token account
- **Code:**
  ```rust
  // src/ctoken/close/processor.rs:60-63
  if compression.info.rent_sponsor != *rent_sponsor.key() {
      return Err(ProgramError::InvalidAccountData);
  }
  ```

##### State Validations

| Check | Location | Error |
|-------|----------|-------|
| Balance == 0 | `processor.rs:44-46` | `NonNativeHasBalance` |
| State != Frozen | `processor.rs:70-74` | `AccountFrozen` |
| State != Uninitialized | `processor.rs:73` | `UninitializedAccount` |
| Destination != token_account | `processor.rs:39-41` | `InvalidAccountData` |

##### Account Closure Procedure (Tip 40 Compliance)

```rust
// src/ctoken/close/processor.rs:197-204
fn finalize_account_closure(accounts: &CloseTokenAccountAccounts<'_>) -> Result<(), ProgramError> {
    unsafe {
        accounts.token_account.assign(&[0u8; 32]); // Reassign to System program
    }
    accounts.token_account.resize(0)?; // Reallocate to 0 bytes
    Ok(())
}
```

##### Security Analysis

| Attack Pattern | Protected | Check Location | Notes |
|----------------|-----------|----------------|-------|
| Owner before read | Y | `check_owner()` | CToken program ownership verified |
| Discriminator | Y | `from_account_info_mut_checked()` | Implicit in zero-copy parse |
| Signer check | Y | `check_signer(authority)` | Authority must sign |
| Account revival | Y | `assign() + resize(0)` | Proper closure procedure |
| TOCTOU | Y | Balance check at close time | Balance verified before close |

---

### 3.2 CToken Operations

#### 3.2.1 CTokenTransfer (Discriminator: 3)

**Source:** `src/ctoken/transfer/default.rs`
**Shared Logic:** `src/ctoken/transfer/shared.rs`
**Enum:** `InstructionType::CTokenTransfer`

##### Account Layout

| Index | Account | Mut | Signer | Checks | Error |
|-------|---------|-----|--------|--------|-------|
| 0 | source | Y | N | Via pinocchio_token_program | SPL errors |
| 1 | destination | Y | N | Via pinocchio_token_program | SPL errors |
| 2 | authority | N | Y | Via pinocchio_token_program + extension validation | SPL/20009 |
| 3* | payer | Y | Y | For top-up if needed | 18061 |

*payer required if source or destination has Compressible extension needing top-up

##### Delegation to pinocchio_token_program

CTokenTransfer delegates core validation to `pinocchio_token_program::processor::transfer::process_transfer()` which performs:
- Source/destination owner check (CToken program)
- Source/destination mint match
- Source balance check
- Authority is owner OR delegate with sufficient delegated_amount
- Source not frozen

##### Extension Validation (shared.rs)

**Sender Validation:**
```rust
// src/ctoken/transfer/shared.rs:152-183
let sender_info = process_account_extensions(source, &mut current_slot, mint)?;

// For restricted extensions, mint is required
if sender_info.flags.has_restricted_extensions() {
    let mint_account = transfer_accounts.mint.ok_or(ErrorCode::MintRequiredForTransfer)?;
    Some(check_mint_extensions(mint_account, deny_restricted_extensions)?)
}
```

**Permanent Delegate Validation:**
```rust
// src/ctoken/transfer/shared.rs:197-214
fn validate_permanent_delegate(mint_checks: Option<&MintExtensionChecks>, authority: &AccountInfo) -> Result<bool, ProgramError> {
    // If permanent_delegate matches authority and is signer -> skip pinocchio validation
    if !authority.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(true)
}
```

**T22 Extension Consistency:**
```rust
// src/ctoken/transfer/shared.rs:32-43
fn check_t22_extensions(&self, other: &Self) -> Result<(), ProgramError> {
    if self.flags.has_pausable != other.flags.has_pausable
        || self.flags.has_permanent_delegate != other.flags.has_permanent_delegate
        || ... {
        Err(ProgramError::InvalidInstructionData)
    }
}
```

##### Security Analysis

| Attack Pattern | Protected | Check Location | Notes |
|----------------|-----------|----------------|-------|
| Owner before read | Y | pinocchio + `from_account_info_mut_checked()` | CToken ownership via pinocchio |
| Signer check | Y | pinocchio + `validate_permanent_delegate()` | Multiple auth paths |
| TOCTOU | Y | Amount in instruction data | Fixed amount at call time |
| Duplicate accounts | Y | pinocchio handles | Same-account transfer checked |

---

#### 3.2.2 CTokenTransferChecked (Discriminator: 12)

**Source:** `src/ctoken/transfer/checked.rs`
**Enum:** `InstructionType::CTokenTransferChecked`

Same as CTokenTransfer with additional:
- Mint account required (not optional)
- Decimals validation via pinocchio_token_program
- Restricted extensions ALLOWED (unlike CTokenTransfer which blocks them)

---

#### 3.2.3 CTokenApprove (Discriminator: 4)

**Source:** `src/ctoken/approve_revoke.rs:28-41`
**Enum:** `InstructionType::CTokenApprove`

##### Account Layout

| Index | Account | Checks | Notes |
|-------|---------|--------|-------|
| 0 | source | Via pinocchio | Token account to approve |
| 1 | delegate | Via pinocchio | Account receiving delegation |
| 2* | payer | Signer + mutable | For compressible top-up |

##### Delegation to pinocchio_token_program

```rust
// src/ctoken/approve_revoke.rs:38-39
process_approve(accounts, &instruction_data[..APPROVE_BASE_LEN])
    .map_err(convert_pinocchio_token_error)?;
```

pinocchio_token_program validates:
- Source account owned by CToken program
- Authority matches source.owner
- Authority is signer
- Source not frozen

---

#### 3.2.4 CTokenRevoke (Discriminator: 5)

**Source:** `src/ctoken/approve_revoke.rs:50-59`
**Enum:** `InstructionType::CTokenRevoke`

##### Account Layout

| Index | Account | Checks | Notes |
|-------|---------|--------|-------|
| 0 | source | Via pinocchio | Token account to revoke |
| 1* | payer | Signer + mutable | For compressible top-up |

##### Delegation to pinocchio_token_program

```rust
process_revoke(accounts).map_err(convert_pinocchio_token_error)?;
```

---

#### 3.2.5 CTokenMintTo (Discriminator: 7)

**Source:** `src/ctoken/mint_to.rs`
**Enum:** `InstructionType::CTokenMintTo`

Delegates to pinocchio_token_program::processor::mint_to::process_mint_to()

Validation:
- Mint owned by Token program
- Authority matches mint.mint_authority
- Authority is signer
- Destination owned by CToken program
- Destination.mint matches mint

---

#### 3.2.6 CTokenMintToChecked (Discriminator: 14)

Same as CTokenMintTo with decimals validation.

---

#### 3.2.7 CTokenBurn (Discriminator: 8)

**Source:** `src/ctoken/burn.rs`
**Enum:** `InstructionType::CTokenBurn`

Delegates to pinocchio_token_program::processor::burn::process_burn()

Validation:
- Source owned by CToken program
- Authority matches source.owner OR source.delegate
- Authority is signer
- Source not frozen
- Sufficient balance/delegated_amount

---

#### 3.2.8 CTokenBurnChecked (Discriminator: 15)

Same as CTokenBurn with decimals validation.

---

#### 3.2.9 CTokenFreezeAccount (Discriminator: 10)

**Source:** `src/ctoken/freeze_thaw.rs`
**Enum:** `InstructionType::CTokenFreezeAccount`

Delegates to pinocchio_token_program::processor::freeze_account::process_freeze_account()

Validation:
- Account owned by CToken program
- Mint owned by Token program
- Authority matches mint.freeze_authority
- Authority is signer
- Account.mint matches mint

---

#### 3.2.10 CTokenThawAccount (Discriminator: 11)

**Source:** `src/ctoken/freeze_thaw.rs`
**Enum:** `InstructionType::CTokenThawAccount`

Delegates to pinocchio_token_program::processor::thaw_account::process_thaw_account()

Same validation as FreezeAccount.

---

### 3.3 Compressed Token Operations

#### 3.3.1 Transfer2 (Discriminator: 101)

**Source:** `src/compressed_token/transfer2/processor.rs`
**Accounts:** `src/compressed_token/transfer2/accounts.rs`
**Enum:** `InstructionType::Transfer2`

This is the most complex instruction supporting:
- Compressed-to-compressed transfers
- Compress (SPL/CToken -> compressed)
- Decompress (compressed -> CToken)
- CompressAndClose (CToken -> compressed + close)

##### Account Layout (varies by mode)

**Mode 1: No compressed accounts (compressions only)**
| Index | Account | Mut | Signer | Checks | Error |
|-------|---------|-----|--------|--------|-------|
| 0 | cpi_authority_pda | N | N | `next_account()` | - |
| 1 | fee_payer | N | Y | `next_signer()` | 20009 |
| 2+ | packed_accounts | - | - | `remaining()` | - |

**Mode 2: With CPI context write**
| Index | Account | Mut | Signer | Checks | Error |
|-------|---------|-----|--------|--------|-------|
| 0 | light_system_program | N | N | `next_non_mut()` | 20005 |
| 1+ | CpiContextLightSystemAccounts | - | - | Various | - |
| N+ | packed_accounts | - | - | `remaining()` | - |

**Mode 3: Standard execution**
| Index | Account | Mut | Signer | Checks | Error |
|-------|---------|-----|--------|--------|-------|
| 0 | light_system_program | N | N | `next_non_mut()` | 20005 |
| 1+ | LightSystemAccounts | - | - | Various | - |
| N+ | packed_accounts | - | - | `remaining()` | - |

##### LightSystemAccounts Validation (`src/shared/accounts.rs`)

| Field | Check | Error |
|-------|-------|-------|
| fee_payer | `next_signer_mut()` | 20002/20009 |
| cpi_authority_pda | `next_non_mut()` | 20005 |
| registered_program_pda | `next_non_mut()` | 20005 |
| account_compression_authority | `next_non_mut()` | 20005 |
| account_compression_program | `next_non_mut()` | 20005 |
| system_program | `next_non_mut()` | 20005 |
| sol_pool_pda (optional) | `next_option()` | - |
| sol_decompression_recipient (optional) | `next_option()` | - |
| cpi_context (optional) | `next_option_mut()` | - |

##### Input Token Data Validation (`src/shared/token_input.rs:30-159`)

```rust
// Index-based account retrieval from packed_accounts
let owner_account = packed_accounts.get(input_token_data.owner as usize)?;
let delegate_account = packed_accounts.get(input_token_data.delegate as usize)?;
let mint_account = packed_accounts.get(input_token_data.mint as usize)?;

// ATA derivation check for is_ata=true (lines 191-238)
if data.is_ata() {
    let wallet_owner = packed_accounts.get(data.owner_index as usize)?;
    let derived_ata = create_program_address(&ata_seeds, &program_id)?;
    if !pubkey_eq(owner_account.key(), &derived_ata) {
        return Err(CTokenError::InvalidAtaDerivation.into());
    }
}

// Authority validation (lines 89-94)
verify_owner_or_delegate_signer(signer_account, delegate_account, permanent_delegate, all_accounts)?;
```

##### Security Analysis

| Attack Pattern | Protected | Check Location | Notes |
|----------------|-----------|----------------|-------|
| Owner before read | Y | token_input.rs via packed_accounts | Index-based, bounds-checked |
| PDA verification | Y | resolve_ata_signer() | ATA derivation verified |
| Signer check | Y | verify_owner_or_delegate_signer() | Multi-auth supported |
| TOCTOU | Y | Amount in instruction data | Fixed at call time |
| Duplicate accounts | N/A | Light system handles | Via CPI |

---

#### 3.3.2 MintAction (Discriminator: 103)

**Source:** `src/compressed_token/mint_action/processor.rs`
**Accounts:** `src/compressed_token/mint_action/accounts.rs`
**Enum:** `InstructionType::MintAction`

Supports 10 action types:
1. CreateCompressedMint
2. MintToCompressed
3. MintToCToken
4. UpdateMintAuthority
5. UpdateFreezeAuthority
6. UpdateMetadataField
7. UpdateMetadataAuthority
8. RemoveMetadataKey
9. DecompressMint
10. CompressAndCloseCMint

##### Account Layout (varies by action)

| Index | Account | Mut | Signer | Condition | Checks |
|-------|---------|-----|--------|-----------|--------|
| 0 | light_system_program | N | N | Always | `next_account()` |
| 1 | mint_signer | N | Conditional* | `with_mint_signer` | `next_option_signer()` or `next_option()` |
| 2 | authority | N | Y | Always | `next_signer()` |
| 3 | compressible_config | N | N | `needs_compressible_accounts()` | `next_config_account()` |
| 4 | cmint | Y | N | `needs_cmint_account()` | `next_option_mut()` |
| 5 | rent_sponsor | Y | N | `needs_compressible_accounts()` | `next_option_mut()` |
| 6+ | LightSystemAccounts | - | - | Not write_to_cpi_context | Various |
| N | out_output_queue | N | N | Not write_to_cpi_context | `next_account()` |
| N+1 | address_merkle_tree OR in_merkle_tree | N | N | Depends on `create_mint` | `next_account()` |
| N+2 | in_output_queue | N | N | Not `create_mint` | `next_option()` |
| N+3 | tokens_out_queue | N | N | `has_mint_to_actions` | `next_option()` |
| N+4+ | packed_accounts (tree + recipient accounts) | - | - | - | `remaining_unchecked()` |

*mint_signer must sign only for CreateCompressedMint, not for DecompressMint

##### Key Validations (`src/compressed_token/mint_action/accounts.rs`)

**mint_signer:**
```rust
// Line 79-83: Signer required for create_mint only
let mint_signer = if config.mint_signer_must_sign() {
    iter.next_option_signer("mint_signer", config.with_mint_signer)?
} else {
    iter.next_option("mint_signer", config.with_mint_signer)?
};
```

**authority:**
```rust
// Line 86: Always required to sign
let authority = iter.next_signer("authority")?;
```

**Address Merkle Tree Validation:**
```rust
// Line 325-333: Must match expected CMINT_ADDRESS_TREE
if let Some(address_tree) = accounts.address_merkle_tree {
    if *address_tree.key() != CMINT_ADDRESS_TREE {
        return Err(ErrorCode::InvalidAddressTree.into());
    }
}
```

**CMint Account Match:**
```rust
// Line 318-322: Verify CMint matches expected pubkey
if let (Some(cmint_account), Some(expected_pubkey)) = (accounts.cmint, cmint_pubkey) {
    if expected_pubkey.to_bytes() != *cmint_account.key() {
        return Err(ErrorCode::MintAccountMismatch.into());
    }
}
```

##### Security Analysis

| Attack Pattern | Protected | Check Location | Notes |
|----------------|-----------|----------------|-------|
| Owner before read | Y | CMint via `zero_copy_at_mut_checked()` | Owner implicit via parse |
| Discriminator | Y | Zero-copy parsing | CMint discriminator checked |
| Signer check | Y | `next_signer()` for authority | Always required |
| PDA verification | Y | `CMINT_ADDRESS_TREE` constant | Fixed address tree |
| CPI program check | Y | Light system hardcoded | Via constant |

---

### 3.4 Compressible Rent Management

#### 3.4.1 Claim (Discriminator: 104)

**Source:** `src/compressible/claim.rs`
**Enum:** `InstructionType::Claim`

##### Account Layout

| Index | Account | Mut | Signer | Checks | Error |
|-------|---------|-----|--------|--------|-------|
| 0 | rent_sponsor | Y | N | `next_mut()` | 20002 |
| 1 | compression_authority | N | Y | `next_signer()` | 20009 |
| 2 | config_account | N | N | `parse_config_account()` | 20001/20000 |
| 3+ | token_accounts | Y | N | `check_owner()` per account | 20001 |

##### Account Details

**[0] rent_sponsor**
- Must match `config_account.rent_sponsor`
- Validation at `claim.rs:45-48`

**[1] compression_authority**
- Must be signer
- Must match `config_account.compression_authority`
- Validation at `claim.rs:41-44`

**[2] config_account**
- Owner: Registry program `Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX`
- Discriminator: `CompressibleConfig::LIGHT_DISCRIMINATOR`
- State: Not INACTIVE (`validate_not_inactive()` at line 37-39)

**[3+] token_accounts (variable)**
- Each validated by `check_owner(&LIGHT_CPI_SIGNER.program_id, account)` at line 114
- Must have Compressible or CMint extension with matching rent_sponsor
- Account type determined by size: 165 bytes = CToken, >165 bytes = check byte 165

##### State Validations (`claim.rs:107-160`)

```rust
// Account type determination (lines 97-105)
fn determine_account_type(data: &[u8]) -> Result<u8, ProgramError> {
    match data.len().cmp(&165) {
        Less => Err(InvalidAccountData),
        Equal => Ok(ACCOUNT_TYPE_TOKEN_ACCOUNT),
        Greater => Ok(data[165])
    }
}

// For CToken accounts
let compressible = ctoken.get_compressible_extension_mut()
    .ok_or(CTokenError::MissingCompressibleExtension)?;

// For CMint accounts
cmint.base.compression.claim_and_update(claim_and_update)?;
```

##### Security Analysis

| Attack Pattern | Protected | Check Location | Notes |
|----------------|-----------|----------------|-------|
| Owner before read | Y | `check_owner()` per token account | Line 114 |
| Discriminator | Y | `zero_copy_at_mut_checked()` | Implicit via parse |
| Signer check | Y | `next_signer()` | compression_authority |
| Authority match | Y | `compression_authority == config.compression_authority` | Line 41-44 |
| Rent sponsor match | Y | `rent_sponsor == config.rent_sponsor` | Line 45-48 |

---

#### 3.4.2 WithdrawFundingPool (Discriminator: 105)

**Source:** `src/compressible/withdraw_funding_pool.rs`
**Enum:** `InstructionType::WithdrawFundingPool`

##### Account Layout

| Index | Account | Mut | Signer | Checks | Error |
|-------|---------|-----|--------|--------|-------|
| 0 | rent_sponsor | Y | N | `next_mut()` + config match | 20002 |
| 1 | compression_authority | N | Y | `next_signer()` + config match | 20009 |
| 2 | destination | Y | N | `next_mut()` | 20002 |
| 3 | system_program | N | N | `next_account()` | - |
| 4 | config | N | N | `parse_config_account()` | 20001/20000 |

##### Account Details

**[0] rent_sponsor**
- Must match `config_account.rent_sponsor`
- PDA derived with seeds `[b"rent_sponsor", version_bytes, bump]`
- Used for `invoke_signed` transfer

**[1] compression_authority**
- Must be signer
- Must match `config_account.compression_authority`
- Validation at line 46-49

**[4] config**
- Owner: Registry program
- Discriminator: CompressibleConfig
- State: Not INACTIVE

##### CPI Verification (Tip 27 Compliance)

```rust
// Lines 110-118: System program transfer via invoke_signed
let transfer = Transfer {
    from: accounts.rent_sponsor,
    to: accounts.destination,
    lamports: amount,
};
transfer.invoke_signed(&[signer]).map_err(convert_program_error)
```

The system program is passed as an account but the `pinocchio_system::Transfer` instruction uses the hardcoded system program ID internally, so CPI program verification is implicit.

##### Security Analysis

| Attack Pattern | Protected | Check Location | Notes |
|----------------|-----------|----------------|-------|
| Owner before read | Y | `parse_config_account()` checks owner | Config account |
| Signer check | Y | `next_signer()` | compression_authority |
| Authority match | Y | config.compression_authority comparison | Line 46-49 |
| Rent sponsor match | Y | config.rent_sponsor comparison | Line 50-53 |
| CPI program check | Y | pinocchio_system hardcoded | Implicit |
| Balance check | Y | `pool_lamports < amount` | Line 91-99 |

---

### 3.5 Token Pool Operations (Anchor)

The following instructions use Anchor framework and are defined in `programs/compressed-token/anchor/src/lib.rs`.

Anchor provides automatic validation through the `#[derive(Accounts)]` macro:
- `#[account(mut)]` - Mutability check
- `#[account(signer)]` - Signer check
- `#[account(owner = X)]` - Owner check
- `#[account(constraint = X)]` - Custom constraint
- `has_one = X` - Field match check
- `seeds = [...]` - PDA derivation check
- `init` - Initialize new account with size/owner

**Note:** Anchor instructions primarily operate on compressed accounts via CPI to the Light System Program. The account validation for compressed account operations is handled by the light-system-program.

#### 3.5.1 CreateTokenPool

**Source:** `programs/compressed-token/anchor/src/lib.rs:50-63`

Creates a token pool for SPL token compression. Each SPL mint can have one primary token pool.

**Accounts:** `CreateTokenPoolInstruction`
- `fee_payer` - Signer, mutable (pays for account)
- `token_pool_pda` - Mutable, initialized via PDA `[b"pool", mint]`
- `mint` - SPL Token/Token-2022 mint
- `system_program` - System program
- `token_program` - SPL Token or Token-2022 program
- `cpi_authority_pda` - PDA authority for the pool

**Validation:**
- Mint extensions checked via `assert_mint_extensions()`
- Token account initialized via CPI to token program

#### 3.5.2 AddTokenPool

**Source:** `programs/compressed-token/anchor/src/lib.rs:68-95`

Creates additional token pools (max 5 per mint).

**Accounts:** `AddTokenPoolInstruction`
- `fee_payer` - Signer, mutable
- `token_pool_pda` - New pool PDA with index
- `existing_token_pool_pda` - Previous pool (must exist)
- `mint` - SPL Token/Token-2022 mint
- `system_program` - System program
- `token_program` - SPL Token or Token-2022 program
- `cpi_authority_pda` - PDA authority

**Validation:**
- `token_pool_index >= NUM_MAX_POOL_ACCOUNTS` (5) -> `InvalidTokenPoolBump`
- Previous pool PDA validated via `is_valid_spl_interface_pda()`

#### 3.5.3 MintTo (Anchor)

**Source:** `programs/compressed-token/anchor/src/lib.rs:104-118`

Mints SPL tokens to compressed accounts.

**Accounts:** `MintToInstruction`
- `fee_payer` - Signer, mutable
- `authority` - Mint authority signer
- `mint` - SPL mint (has_one = authority)
- `token_pool_pda` - Token pool
- `token_program` - Token program
- Remaining accounts for Light System CPI

**Validation:**
- Authority must match mint.mint_authority (Anchor `has_one`)
- Tokens transferred to pool, compressed equivalents created

#### 3.5.4 BatchCompress

**Source:** `programs/compressed-token/anchor/src/lib.rs:121-146`

Batch compress tokens to multiple recipients.

**Accounts:** Same as `MintToInstruction`

**Validation:**
- Cannot have both `amounts` and `amount` in instruction data -> `AmountsAndAmountProvided`
- Must have either `amounts` or `amount` -> `NoAmount`

#### 3.5.5 CompressSplTokenAccount

**Source:** `programs/compressed-token/anchor/src/lib.rs:151-158`

Compresses SPL token account balance to compressed tokens.

**Accounts:** `TransferInstruction`
- `fee_payer` - Signer, mutable
- `authority` - Token account authority signer
- `compress_or_decompress_token_account` - SPL token account
- `token_pool_pda` - Token pool
- `token_program` - Token program
- Remaining accounts for Light System CPI

**Validation:**
- Authority must be owner of compress_or_decompress_token_account
- Sufficient balance for compression

#### 3.5.6 Transfer (Anchor)

**Source:** `programs/compressed-token/anchor/src/lib.rs:168-182`

Transfers compressed tokens with optional compression/decompression.

**Accounts:** `TransferInstruction`

**Validation:**
- CPI context validated if compression/decompression (`check_cpi_context()`)
- Sum checks performed (inputs = outputs + compression/decompression)

#### 3.5.7 Approve (Anchor)

**Source:** `programs/compressed-token/anchor/src/lib.rs:190-195`

Delegates compressed tokens.

**Accounts:** `GenericInstruction`
- Standard Light System accounts for compressed account operations

**Validation:**
- Owner must sign (via compressed account proof)
- Creates delegated output + change output

#### 3.5.8 Revoke (Anchor)

**Source:** `programs/compressed-token/anchor/src/lib.rs:199-204`

Revokes delegation on compressed tokens.

**Accounts:** `GenericInstruction`

**Validation:**
- Owner must sign (not delegate)
- Merges all inputs into single undelegated output

#### 3.5.9 Freeze (Anchor)

**Source:** `programs/compressed-token/anchor/src/lib.rs:208-214`

Freezes compressed token accounts.

**Accounts:** `FreezeInstruction`
- `authority` - Freeze authority signer
- `mint` - SPL mint with freeze authority
- Remaining accounts for Light System CPI

**Validation:**
- Input accounts must NOT be frozen
- Authority must match mint.freeze_authority

#### 3.5.10 Thaw (Anchor)

**Source:** `programs/compressed-token/anchor/src/lib.rs:218-224`

Thaws frozen compressed token accounts.

**Accounts:** `FreezeInstruction`

**Validation:**
- Input accounts must BE frozen
- Authority must match mint.freeze_authority

#### 3.5.11 Burn (Anchor)

**Source:** `programs/compressed-token/anchor/src/lib.rs:229-234`

Burns compressed tokens.

**Accounts:** `BurnInstruction`
- `authority` - Owner or delegate signer
- `mint` - SPL mint
- `token_pool_pda` - Token pool (for SPL token burn)
- `token_program` - Token program
- Remaining accounts for Light System CPI

**Validation:**
- Delegates can burn (output remains delegated)
- SPL tokens burned from pool account

---

## 4. Potential Gaps and Recommendations

During this security review, the following areas were identified for potential hardening:

### 4.1 Packed Accounts Owner Validation

**Location:** `src/shared/token_input.rs`, `src/compressed_token/transfer2/accounts.rs`

**Observation:** Accounts retrieved from `packed_accounts` by index (owner, delegate, mint) do not have explicit `check_owner()` calls at retrieval time. Validation relies on zero-copy parsing to fail if data is invalid.

**Current Protection:** Implicit via CToken/CMint deserialization checks.

**Recommendation:** Consider adding explicit owner checks for mint accounts before parsing extensions.

### 4.2 Tree Account Identification Heuristic

**Location:** `src/compressed_token/transfer2/accounts.rs:147-155`

```rust
// Checks first 8 bytes of owner (account-compression program prefix)
if account_info.owner()[0..8] == [9, 44, 54, 236, 34, 245, 23, 131] {
```

**Status:** Acceptable - the 8-byte prefix of the account-compression program ID is unique and a collision is practically impossible. This optimization reduces compute cost without sacrificing security.

### 4.3 Non-Compressible CreateTokenAccount

**Location:** `src/ctoken/create.rs:49`

**Observation:** In non-compressible path, `token_account` fetched via `next_mut()` without explicit owner check. Comment states "ownership is implicitly validated when writing."

**Current Protection:** Solana runtime enforces owner-only writes.

**Status:** Acceptable - runtime protection adequate.

### 4.4 Areas with Strong Protection

The following areas have robust account validation:

1. **CloseTokenAccount** - Proper `check_owner()`, `check_signer()`, and account revival prevention (`assign()` + `resize(0)`)

2. **Config Account Validation** - Full owner + discriminator + state validation via `parse_config_account()`

3. **Authority Validation** - `verify_owner_or_delegate_signer()` covers owner, delegate, and permanent delegate with proper signer checks

4. **ATA Derivation** - `resolve_ata_signer()` in token_input.rs properly validates PDA derivation

5. **CPI Safety** - All CPI calls use hardcoded program IDs (light_system_program, pinocchio_system, pinocchio_token_program)

---

## 5. Quick Reference Tables

### 5.1 Account Checks by Instruction

| Instruction | Disc | Accounts | Signers | Owner Checks | PDA Checks |
|-------------|------|----------|---------|--------------|------------|
| CreateTokenAccount | 18 | 2-6 | 1-2 | config | compress_to_pubkey |
| CreateAssociatedCTokenAccount | 100 | 5-7 | 1 | system, config | ATA derivation |
| CreateAssociatedTokenAccountIdempotent | 102 | 5-7 | 1 | system/ctoken, config | ATA derivation |
| CloseTokenAccount | 9 | 3-4 | 1 | ctoken | - |
| CTokenTransfer | 3 | 3-4 | 1 | ctoken (via pinocchio) | - |
| CTokenTransferChecked | 12 | 4-5 | 1 | ctoken, token (via pinocchio) | - |
| CTokenApprove | 4 | 2-3 | 1 | ctoken (via pinocchio) | - |
| CTokenRevoke | 5 | 1-2 | 1 | ctoken (via pinocchio) | - |
| CTokenMintTo | 7 | 3-4 | 1 | token, ctoken (via pinocchio) | - |
| CTokenMintToChecked | 14 | 3-4 | 1 | token, ctoken (via pinocchio) | - |
| CTokenBurn | 8 | 3-4 | 1 | token, ctoken (via pinocchio) | - |
| CTokenBurnChecked | 15 | 3-4 | 1 | token, ctoken (via pinocchio) | - |
| CTokenFreezeAccount | 10 | 3 | 1 | token, ctoken (via pinocchio) | - |
| CTokenThawAccount | 11 | 3 | 1 | token, ctoken (via pinocchio) | - |
| Transfer2 | 101 | Variable | 1+ | ctoken, light_system | Various |
| MintAction | 103 | Variable | 1+ | ctoken, light_system | Various |
| Claim | 104 | 3+ | 1 | ctoken, registry | - |
| WithdrawFundingPool | 105 | 5 | 1 | registry | rent_sponsor PDA |

### 5.2 Security Checklist Summary

| Attack Pattern | Protection Method | Key Locations |
|----------------|-------------------|---------------|
| Owner before read | `check_owner()`, pinocchio validation | All instruction entry points |
| Discriminator | `check_discriminator()`, zero-copy parse | Account deserialization |
| Signer verification | `check_signer()`, `next_signer*()` | Authority accounts |
| PDA verification | `check_pda_seeds()`, `validate_ata_derivation()` | ATA, rent_sponsor |
| Account revival | `assign()` + `resize(0)` | CloseTokenAccount |
| TOCTOU | Amount in instruction data | Transfer, Burn, Approve |
| CPI program check | pinocchio_token_program hardcoded | All CPI calls |
| Duplicate accounts | pinocchio handles | Transfer operations |

---

## Appendix

### A. File Reference

**Validation Primitives:**
- `program-libs/account-checks/src/checks.rs`
- `program-libs/account-checks/src/account_iterator.rs`
- `program-libs/account-checks/src/error.rs`

**Token-Specific Validation:**
- `programs/compressed-token/program/src/shared/owner_validation.rs`
- `programs/compressed-token/program/src/shared/token_input.rs`
- `programs/compressed-token/program/src/shared/config_account.rs`
- `programs/compressed-token/program/src/extensions/check_mint_extensions.rs`

**Instruction Processors:**
- `programs/compressed-token/program/src/ctoken/` - CToken operations
- `programs/compressed-token/program/src/compressed_token/` - Compressed operations
- `programs/compressed-token/program/src/compressible/` - Rent management
- `programs/compressed-token/anchor/src/lib.rs` - Anchor instructions

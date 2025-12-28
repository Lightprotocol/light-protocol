## CToken Freeze Account

**discriminator:** 10
**enum:** `CTokenInstruction::CTokenFreezeAccount`
**path:** programs/compressed-token/program/src/ctoken_freeze_thaw.rs

**description:**
Freezes a decompressed ctoken account, preventing transfers and other operations while frozen. This is a pass-through instruction that validates mint ownership (must be owned by SPL Token, Token-2022, or CToken program) before delegating to pinocchio-token-program for standard SPL Token freeze validation. After freezing, the account's state field is set to AccountState::Frozen, and only the freeze_authority of the mint can freeze accounts (mint must have freeze_authority set). The account layout `CToken` is defined in program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs.

**Instruction data:**
No instruction data required beyond the discriminator byte.

**Accounts:**
1. token_account
   - (mutable)
   - The ctoken account to freeze
   - Must be initialized (AccountState::Initialized)
   - Will have state field updated to AccountState::Frozen

2. mint
   - The mint account associated with the token account
   - Must be owned by SPL Token, Token-2022, or CToken program
   - Must have freeze_authority set (not None)

3. freeze_authority
   - (signer)
   - Must match the mint's freeze_authority
   - Must sign the transaction

**Instruction Logic and Checks:**

1. **Validate minimum accounts:**
   - Require at least 2 accounts to get mint account (index 1)
   - Return NotEnoughAccountKeys if insufficient

2. **Validate mint ownership:**
   - Get mint account (accounts[1])
   - Call `check_token_program_owner(mint_info)` from programs/compressed-token/program/src/shared/owner_validation.rs
   - Verify mint is owned by one of:
     - SPL Token program (TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA)
     - Token-2022 program (TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb)
     - CToken program (cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m)
   - Return IncorrectProgramId if mint owner doesn't match

3. **Delegate to pinocchio-token-program:**
   - Call `process_freeze_account(accounts)` from pinocchio-token-program
   - This performs standard SPL Token freeze validation:
     - Verifies token_account is mutable
     - Verifies freeze_authority is signer
     - Verifies token_account.mint == mint.key()
     - Verifies mint.freeze_authority == Some(freeze_authority.key())
     - Verifies token_account state is Initialized (not already Frozen)
     - Updates token_account.state to AccountState::Frozen
   - Map any errors from u64 to ProgramError::Custom(u32)

**Errors:**
- `ProgramError::NotEnoughAccountKeys` (error code: 11) - Less than 2 accounts provided (cannot get mint account)
- `ProgramError::IncorrectProgramId` (error code: 7) - Mint is not owned by a valid token program (SPL Token, Token-2022, or CToken)
- SPL Token errors from pinocchio-token-program (converted from u64 to ProgramError::Custom(u32)):
  - `TokenError::MintCannotFreeze` (error code: 16) - Mint's freeze_authority is None
  - `TokenError::OwnerMismatch` (error code: 4) - freeze_authority doesn't match mint's freeze_authority
  - `TokenError::MintMismatch` (error code: 3) - token_account's mint doesn't match provided mint
  - `TokenError::InvalidState` (error code: 13) - Account is already frozen or uninitialized
  - `ProgramError::InvalidAccountData` (error code: 4) - Account data is malformed

## Comparison with Token-2022

### Functional Parity

CToken's FreezeAccount instruction maintains complete functional parity with Token-2022 for core freeze operations:

- **Same discriminator:** Both use discriminator 10 (0x0A)
- **Same account requirements:** token_account (writable), mint (read-only), freeze_authority (signer)
- **Same state transitions:** Initialized → Frozen (prevents reverse transition Frozen → Frozen)
- **Same authority validation:** Verifies freeze_authority matches mint's freeze_authority
- **Same error handling:** Returns identical TokenError codes (MintCannotFreeze, OwnerMismatch, MintMismatch, InvalidState)
- **Extension support:** Both handle Token-2022 extensions through TLV unpacking (PodStateWithExtensionsMut)

### CToken-Specific Features

**Additional Mint Ownership Validation:**
CToken adds an explicit mint ownership check before delegating to the standard freeze logic:

```rust
// programs/compressed-token/program/src/ctoken_freeze_thaw.rs:14-15
let mint_info = accounts.get(1).ok_or(ProgramError::NotEnoughAccountKeys)?;
check_token_program_owner(mint_info)?;
```

This validates that the mint is owned by one of:
- SPL Token program (TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA)
- Token-2022 program (TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb)
- CToken program (cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m)

**Security benefit:** This explicit check provides defense-in-depth by failing fast with `ProgramError::IncorrectProgramId` before attempting deserialization, preventing potential cross-program account confusion.

**Comparison with Token-2022:** Token-2022 relies on implicit validation through `PodStateWithExtensions::unpack()` which would fail on invalid mint data, but does not perform explicit ownership validation (see Token-2022 analysis: "MISSING CHECK 2: Mint Program Ownership").

### Missing Features

**No Multisig Support:**
CToken's freeze instruction does not support multisig freeze authorities. The instruction only accepts:
- Single signer freeze authority (accounts[2] must be signer)

Token-2022 supports both:
- Single owner: 3 accounts (token_account, mint, freeze_authority)
- Multisig owner: 3+M accounts (token_account, mint, multisig_account, ...M signers)

**Impact:** Mints with multisig freeze authorities cannot use CToken freeze operations. Users must rely on the native Token-2022 freeze instruction for multisig-controlled mints.

### Extension Handling Differences

**Token-2022 Extensions:**
Both CToken and Token-2022 handle extensions identically through the underlying `process_freeze_account` implementation:
- Uses `PodStateWithExtensionsMut::<PodAccount>::unpack()` for token account
- Uses `PodStateWithExtensions::<PodMint>::unpack()` for mint
- No extension-specific validation required (freeze operates on base state only)

**CToken-Specific Extensions:**
CToken accounts may have a `Compressible` extension (not present in SPL/Token-2022). The freeze instruction operates on the base `CToken` state and does not interact with the compressible extension. Frozen accounts remain frozen after compression/decompression cycles.

**Permanent Delegate Interaction:**
- Token-2022: Permanent delegate cannot transfer/burn from frozen accounts (operations fail with AccountFrozen)
- CToken: Same behavior - permanent delegate cannot compress frozen accounts (frozen check in `programs/compressed-token/program/src/transfer2/compression/ctoken/compress_or_decompress_ctokens.rs:173-178`)

**Default Account State Extension:**
- Token-2022: Supports `DefaultAccountState` extension to create accounts in frozen state by default
- CToken: Supports this extension when creating CToken accounts from Token-2022 mints (extension data preserved during decompression)

### Security Property Comparison

Both implementations provide equivalent security properties:

| Security Property | Token-2022 | CToken |
|------------------|------------|---------|
| Account initialization validation | Yes (unpack checks is_initialized) | Yes (via pinocchio-token-program) |
| Account type validation | Yes (checks AccountType::Account) | Yes (via pinocchio-token-program) |
| State transition guards | Yes (prevents Frozen→Frozen) | Yes (via pinocchio-token-program) |
| Native account rejection | Yes (NativeNotSupported) | Yes (via pinocchio-token-program) |
| Mint association validation | Yes (key comparison) | Yes (via pinocchio-token-program) |
| Mint initialization validation | Yes (unpack checks is_initialized) | Yes (via pinocchio-token-program) |
| Freeze authority existence check | Yes (checks PodCOption::SOME) | Yes (via pinocchio-token-program) |
| Freeze authority key validation | Yes (validate_owner) | Yes (via pinocchio-token-program) |
| Single signer validation | Yes | Yes (via pinocchio-token-program) |
| Multisig support | Yes (M-of-N threshold) | No |
| **Explicit mint ownership check** | **No** (implicit via unpack) | **Yes** (explicit check_token_program_owner) |
| **Explicit account ownership check** | **No** (implicit via unpack) | **No** (implicit via unpack) |

**Key Differences:**
1. **CToken adds explicit mint ownership validation** - Provides defense-in-depth with clear error messages before data borrowing
2. **Token-2022 supports multisig** - CToken only supports single signer freeze authorities
3. **Both lack explicit account ownership validation** - Rely on implicit unpack failures for non-token-program accounts

### Implementation Architecture

**Token-2022:**
```
FreezeAccount instruction (discriminator: 10)
  ↓
process_toggle_freeze_account(freeze=true)
  ↓
- Unpack source account (PodStateWithExtensionsMut)
- Unpack mint (PodStateWithExtensions)
- Validate freeze authority (single or multisig)
- Update account state to Frozen
```

**CToken:**
```
CTokenFreezeAccount instruction (discriminator: 10)
  ↓
process_ctoken_freeze_account()
  ↓
check_token_program_owner(mint) // Additional validation
  ↓
process_freeze_account() (from pinocchio-token-program)
  ↓
- Same validation logic as Token-2022 single-signer path
- Update account state to Frozen
```

**Architecture benefit:** CToken reuses Token-2022's battle-tested freeze logic through pinocchio-token-program while adding an extra layer of mint ownership validation.

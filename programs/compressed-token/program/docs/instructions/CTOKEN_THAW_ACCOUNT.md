## CToken Thaw Account

**discriminator:** 11
**enum:** `InstructionType::CTokenThawAccount`
**path:** programs/compressed-token/program/src/ctoken_freeze_thaw.rs

**description:**
Thaws a frozen decompressed ctoken account, restoring normal operation. This is a pass-through instruction that validates mint ownership (must be owned by SPL Token, Token-2022, or CToken program) before delegating to pinocchio-token-program for standard SPL Token thaw validation. After thawing, the account's state field is set to AccountState::Initialized, and only the freeze_authority of the mint can thaw accounts (mint must have freeze_authority set). The account layout `CToken` is defined in program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs.

**Instruction data:**
No instruction data required beyond the discriminator byte.

**Accounts:**
1. token_account
   - (mutable)
   - The frozen ctoken account to thaw
   - Must be frozen (AccountState::Frozen)
   - Will have state field updated to AccountState::Initialized

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

3. **Process thaw (inline via pinocchio-token-program library):**
   - Call `process_thaw_account(accounts)` from pinocchio-token-program
   - This performs standard SPL Token thaw validation:
     - Verifies token_account is mutable
     - Verifies freeze_authority is signer
     - Verifies token_account.mint == mint.key()
     - Verifies mint.freeze_authority == Some(freeze_authority.key())
     - Verifies token_account state is Frozen (not already Initialized)
     - Updates token_account.state to AccountState::Initialized
   - Map any errors from u64 to ProgramError::Custom(u32)

**Errors:**
- `ProgramError::NotEnoughAccountKeys` (error code: 11) - Less than 2 accounts provided (cannot get mint account)
- `ProgramError::IncorrectProgramId` (error code: 7) - Mint is not owned by a valid token program (SPL Token, Token-2022, or CToken)
- SPL Token errors from pinocchio-token-program (converted from u64 to ProgramError::Custom(u32)):
  - `TokenError::MintCannotFreeze` (error code: 16) - Mint's freeze_authority is None
  - `TokenError::OwnerMismatch` (error code: 4) - freeze_authority doesn't match mint's freeze_authority
  - `TokenError::MintMismatch` (error code: 3) - token_account's mint doesn't match provided mint
  - `TokenError::InvalidState` (error code: 13) - Account is not frozen or is uninitialized
  - `ProgramError::InvalidAccountData` (error code: 4) - Account data is malformed

## Comparison with SPL Token

### Functional Parity

CToken delegates core logic to `pinocchio_token_program::processor::thaw_account::process_thaw_account`, which implements SPL Token-compatible thaw semantics:
- State transition (Frozen → Initialized), freeze authority validation, mint association check

### CToken-Specific Features

**1. Explicit Mint Ownership Validation**
CToken adds `check_token_program_owner(mint)` before delegating to thaw logic, validating mint is owned by SPL Token, Token-2022, or CToken program.

### Unsupported SPL & Token-2022 Features

**1. No Multisig Support**
**2. No CPI Guard Extension Check**

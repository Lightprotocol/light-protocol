## CToken Thaw Account

**discriminator:** 11
**enum:** `InstructionType::CTokenThawAccount`
**path:** programs/compressed-token/program/src/ctoken/freeze_thaw.rs

**description:**
Thaws a frozen decompressed ctoken account, restoring normal operation. This is a pass-through instruction that validates mint ownership (must be owned by SPL Token, Token-2022, or CToken program) before delegating to pinocchio-token-program for standard SPL Token thaw validation. After thawing, the account's state field is set to AccountState::Initialized, and only the freeze_authority of the mint can thaw accounts (mint must have freeze_authority set). The account layout `CToken` is defined in program-libs/token-interface/src/state/ctoken/ctoken_struct.rs.

**Instruction data:**
No instruction data required beyond the discriminator byte.

**Accounts:**
1. token_account
   - (mutable)
   - The frozen ctoken account to thaw
   - Must be frozen (AccountState::Frozen)
   - Must not be a native token account
   - Will have state field updated to AccountState::Initialized

2. mint
   - The mint account associated with the token account
   - Must be owned by SPL Token, Token-2022, or CToken program
   - Must have freeze_authority set (not None)
   - Must match token_account.mint

3. freeze_authority
   - (signer, or multisig with signers in remaining accounts)
   - Must match the mint's freeze_authority
   - Must sign the transaction (or provide sufficient multisig signers)

4. remaining accounts (optional)
   - Additional signer accounts if freeze_authority is a multisig

**Instruction Logic and Checks:**

1. **Validate minimum accounts (CToken layer):**
   - Require at least 2 accounts to access mint account (index 1)
   - Return NotEnoughAccountKeys if insufficient

2. **Validate mint ownership (CToken layer):**
   - Get mint account (accounts[1])
   - Call `check_token_program_owner(mint_info)` from programs/compressed-token/program/src/shared/owner_validation.rs
   - Verify mint is owned by one of:
     - SPL Token program (TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA)
     - Token-2022 program (TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb)
     - CToken program (cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m)
   - Return IncorrectProgramId if mint owner doesn't match

3. **Process thaw (pinocchio-token-program layer):**
   - Call `process_thaw_account(accounts)` from pinocchio-token-program
   - Internally calls `process_toggle_account_state(accounts, false)` which:
     - Requires at least 3 accounts: [source_account, mint, authority, remaining...]
     - Loads token account mutably and validates it is initialized
     - Verifies token_account state is Frozen (returns InvalidState if already Initialized)
     - Verifies token_account is not a native token (returns NativeNotSupported)
     - Verifies token_account.mint == mint.key() (returns MintMismatch)
     - Loads mint and verifies freeze_authority is set (returns MintCannotFreeze if None)
     - Validates owner via `validate_owner()`:
       - Checks freeze_authority key matches expected authority
       - If authority is a multisig account, validates sufficient signers from remaining accounts
       - If authority is a regular account, verifies it is a signer
     - Updates token_account.state to AccountState::Initialized
   - Errors are converted via `convert_pinocchio_token_error` to anchor ErrorCode variants

**Errors:**
- `ProgramError::NotEnoughAccountKeys` - Less than 2 accounts provided (CToken check), or less than 3 accounts for pinocchio processor
- `ProgramError::IncorrectProgramId` - Mint is not owned by a valid token program (SPL Token, Token-2022, or CToken)
- SPL Token errors from pinocchio-token-program (converted to anchor ErrorCode variants):
  - `ErrorCode::MintHasNoFreezeAuthority` (SPL code 16) - Mint's freeze_authority is None
  - `ErrorCode::OwnerMismatch` (SPL code 4) - freeze_authority doesn't match mint's freeze_authority
  - `ErrorCode::MintMismatch` (SPL code 3) - token_account's mint doesn't match provided mint
  - `ErrorCode::InvalidState` (SPL code 13) - Account is not frozen (already Initialized or uninitialized)
  - `ErrorCode::NativeNotSupported` (SPL code 10) - Cannot thaw native token accounts
  - `ProgramError::MissingRequiredSignature` - Authority is not a signer or multisig threshold not met

## Comparison with SPL Token

### Functional Parity

CToken delegates core logic to `pinocchio_token_program::processor::thaw_account::process_thaw_account`, which implements SPL Token-compatible thaw semantics:
- State transition (Frozen -> Initialized), freeze authority validation, mint association check

### CToken-Specific Features

**1. Explicit Mint Ownership Validation**
CToken adds `check_token_program_owner(mint)` before delegating to thaw logic, validating mint is owned by SPL Token, Token-2022, or CToken program. This allows CToken mints to be thawed as well as standard SPL/Token-2022 mints.

### Supported SPL Features

**1. Multisig Support**
The pinocchio-token-program implementation supports multisig freeze authorities. If the freeze_authority is a multisig account, additional signer accounts can be passed in the remaining accounts to meet the signature threshold.

### Unsupported Token-2022 Features

**1. No CPI Guard Extension Check**
Token-2022's CPI guard extension check is not performed.

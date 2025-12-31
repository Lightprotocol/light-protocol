## CToken ApproveChecked

**discriminator:** 13
**enum:** `InstructionType::CTokenApproveChecked`
**path:** programs/compressed-token/program/src/ctoken_approve_revoke.rs

**description:**
Delegates a specified amount to a delegate authority on a decompressed ctoken account with decimals validation, fully compatible with SPL Token ApproveChecked semantics. Account layout `CToken` is defined in program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs. Extension layout `CompressionInfo` is defined in program-libs/compressible/src/compression_info.rs. Uses pinocchio-token-program to process the approve operation. Before the approve operation, automatically tops up compressible accounts with additional lamports if needed to prevent accounts from becoming compressible during normal operations. Supports max_top_up parameter (0 = no limit) that enforces transaction failure if the calculated top-up exceeds this limit. Uses cached decimals optimization: if source CToken has cached decimals, validates against instruction decimals and skips mint read. Cached decimals allow users to choose whether a cmint is required to be decompressed at account creation or transfer.

**Instruction data:**
Path: programs/compressed-token/program/src/ctoken_approve_revoke.rs (lines 163-217)

- Bytes 0-7: `amount` (u64, little-endian) - Number of tokens to delegate
- Byte 8: `decimals` (u8) - Expected token decimals
- Bytes 9-10 (optional): `max_top_up` (u16, little-endian) - Maximum lamports for top-up (0 = no limit)

Format variants:
- 9 bytes: amount + decimals (legacy, no max_top_up enforcement)
- 11 bytes: amount + decimals + max_top_up

**Accounts:**
1. source
   - (mutable)
   - The source ctoken account to approve delegation on
   - May receive rent top-up if compressible
   - May have cached decimals for validation optimization

2. mint
   - (immutable)
   - The mint account for the token
   - Must match source account's mint
   - Decimals field must match instruction data decimals parameter
   - Only read if source account has no cached decimals

3. delegate
   - (immutable)
   - The delegate authority who will be granted spending rights
   - Does not need to sign

4. owner
   - (signer, mutable)
   - Owner of the source account
   - Must sign the transaction
   - Acts as payer for rent top-up if compressible extension present

**Instruction Logic and Checks:**

1. **Validate minimum accounts:**
   - Require at least 4 accounts (source, mint, delegate, owner)
   - Return NotEnoughAccountKeys if insufficient

2. **Parse instruction data:**
   - Require at least 9 bytes (amount + decimals)
   - Parse amount (u64) and decimals (u8) using unpack_amount_and_decimals
   - If 11 bytes: parse max_top_up from bytes 9-10
   - If 9 bytes: set max_top_up = 0 (no limit)
   - Return InvalidInstructionData for any other length

3. **Get cached decimals and process compressible top-up:**
   - Borrow source account data mutably
   - Deserialize CToken using zero-copy validation
   - Get cached decimals via `ctoken.base.decimals()` (returns Option<u8>)
   - Initialize lamports_budget based on max_top_up:
     - If max_top_up == 0: budget = u64::MAX (no limit)
     - Otherwise: budget = max_top_up + 1 (allows exact match)
   - Call process_compression_top_up with source account's compression info
   - Drop borrow before CPI
   - If transfer_amount > 0:
     - Check that transfer_amount <= lamports_budget
     - Return MaxTopUpExceeded if budget exceeded
     - Transfer lamports from owner to source via CPI

4. **Process SPL approve based on cached decimals:**
   - **If cached decimals present:**
     - Validate cached_decimals == instruction decimals
     - Return InvalidInstructionData if mismatch
     - Create 3-account slice [source, delegate, owner] (skip mint)
     - Call process_approve with expected_decimals = None (skip pinocchio mint validation)
   - **If no cached decimals:**
     - Validate mint is owned by valid token program (SPL, Token-2022, or CToken)
     - Call process_approve with full 4-account layout and expected_decimals = Some(decimals)

**Errors:**

- `ProgramError::NotEnoughAccountKeys` (error code: 11) - Less than 4 accounts provided
- `ProgramError::InvalidInstructionData` (error code: 3) - Instruction data is not 9 or 11 bytes, or cached decimals != instruction decimals
- `ProgramError::IncorrectProgramId` (error code: 7) - Mint is not owned by a valid token program (when no cached decimals)
- `CTokenError::InvalidAccountData` (error code: 18002) - Failed to deserialize CToken account
- `CTokenError::SysvarAccessError` (error code: 18020) - Failed to get Clock or Rent sysvar for top-up calculation
- `CTokenError::MaxTopUpExceeded` (error code: 18043) - Calculated top-up exceeds max_top_up parameter
- `ProgramError::MissingRequiredSignature` (error code: 8) - Owner did not sign the transaction (SPL Token error)
- Pinocchio token errors (converted to ProgramError::Custom):
  - `TokenError::OwnerMismatch` (error code: 4) - Authority doesn't match account owner
  - `TokenError::AccountFrozen` (error code: 17) - Account is frozen
  - `TokenError::MintMismatch` (error code: 3) - Mint doesn't match source account's mint
  - `TokenError::MintDecimalsMismatch` (error code: 18) - Decimals don't match mint's decimals

## Comparison with Token-2022

### Functional Parity

CToken ApproveChecked maintains compatibility with SPL Token-2022's ApproveChecked:

- **Delegate Authorization**: Both delegate spending authority to a delegate pubkey for a specified token amount
- **Owner Signature**: Transaction must be signed by the account owner (single owner only, no multisig support in CToken)
- **Account State Validation**: Both check that the source account is initialized and not frozen
- **Decimals Validation**: Both validate instruction decimals against mint decimals

### CToken-Specific Features

1. **Cached Decimals Optimization**: If source CToken has cached decimals, validates against instruction and skips mint read
2. **Compressible Top-Up Logic**: Automatically tops up accounts with the Compressible extension
3. **max_top_up Parameter**: Limits rent top-up costs (0 = no limit)
4. **Static 4-Account Layout**: Always requires mint account, but may skip reading it when cached decimals are available


### Unsupported SPL & Token-2022 Features

**1. No Multisig Support**
**2. No CPI Guard Extension Check**

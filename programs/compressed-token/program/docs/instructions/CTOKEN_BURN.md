## CToken Burn

**discriminator:** 8
**enum:** `InstructionType::CTokenBurn`
**path:** programs/compressed-token/program/src/ctoken_burn.rs

**description:**
Burns tokens from a decompressed CToken account and decreases the CMint supply, fully compatible with SPL Token burn semantics. Account layout `CToken` is defined in `program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs`. Account layout `CompressedMint` (CMint) is defined in `program-libs/ctoken-interface/src/state/mint/compressed_mint.rs`. Extension layout `CompressionInfo` is defined in `program-libs/compressible/src/compression_info.rs` and is embedded in both CToken and CMint structs. Uses pinocchio-token-program to process the burn (handles balance/supply updates, authority check, frozen check). After the burn, automatically tops up compressible accounts with additional lamports if needed. Top-up is calculated for both CMint and source CToken based on current slot and account balance. Top-up prevents accounts from becoming compressible during normal operations. Enforces max_top_up limit if provided (transaction fails if exceeded). Supports max_top_up parameter to limit rent top-up costs (0 = no limit). Instruction data is backwards-compatible: 8-byte format (legacy, no max_top_up enforcement) and 10-byte format (with max_top_up). This instruction only works with CMints (compressed mints). CMints do not support restricted Token-2022 extensions (Pausable, TransferFee, TransferHook, PermanentDelegate, DefaultAccountState) - only TokenMetadata is allowed. To burn tokens from spl or T22 mints, use Transfer2 with decompress mode to convert to SPL tokens first, then burn via SPL Token-2022.

**Instruction data:**

Format 1 (8 bytes, legacy):
- Bytes 0-7: `amount` (u64, little-endian) - Number of tokens to burn
- No max_top_up enforcement (effectively unlimited)

Format 2 (10 bytes):
- Bytes 0-7: `amount` (u64, little-endian) - Number of tokens to burn
- Bytes 8-9: `max_top_up` (u16, little-endian) - Maximum lamports for combined CMint + CToken top-ups (0 = no limit)

**Accounts:**
1. source CToken
   - (mutable)
   - The CToken account to burn from
   - Must have sufficient balance for the burn
   - May receive rent top-up if compressible
   - Must not be frozen

2. CMint
   - (mutable)
   - The compressed mint account
   - Supply is decreased by burn amount
   - May receive rent top-up if compressible

3. authority
   - (signer)
   - Owner of the source CToken account
   - Must sign the transaction
   - Also serves as payer for rent top-ups if needed

**Instruction Logic and Checks:**

1. **Validate minimum accounts:**
   - Require at least 3 accounts (source CToken, CMint, authority/payer)
   - Return NotEnoughAccountKeys if insufficient

2. **Parse instruction data:**
   - Require at least 8 bytes for amount
   - Parse max_top_up:
     - If instruction_data.len() == 8: max_top_up = 0 (no limit, legacy format)
     - If instruction_data.len() == 10: parse u16 from bytes 8-9 as max_top_up
     - Otherwise: return InvalidInstructionData

3. **Process SPL burn via pinocchio-token-program:**
   - Call `process_burn` with first 8 bytes (amount only)
   - Validates authority signature matches source CToken owner
   - Checks source CToken balance is sufficient for burn amount
   - Checks source CToken is not frozen
   - Decreases source CToken balance by amount
   - Decreases CMint supply by amount
   - Errors are converted from pinocchio errors to ProgramError::Custom

4. **Calculate and execute top-up transfers:**
   Called via `calculate_and_execute_compressible_top_ups(cmint, ctoken, payer, max_top_up)`:

   a. **Initialize transfer array and budget:**
      - Create Transfer array for [cmint, ctoken] with amounts initialized to 0
      - Initialize lamports_budget to max_top_up + 1 (allowing exact match when total == max_top_up)

   b. **Calculate CMint top-up:**
      - Borrow CMint data and deserialize using `CompressedMint::zero_copy_at`
      - Access compression info directly from mint.base.compression (embedded in all CMints)
      - Lazy load Clock sysvar for current_slot and Rent sysvar if not yet loaded (current_slot == 0)
      - Call `compression.calculate_top_up_lamports(data_len, current_slot, lamports, rent_exemption)`
      - Subtract calculated top-up from lamports_budget

   c. **Calculate CToken top-up:**
      - Skip if CToken data length is 165 bytes (no extensions, standard SPL token account)
      - Borrow CToken data and deserialize using `CToken::zero_copy_at_checked`
      - Get Compressible extension via `token.get_compressible_extension()`
      - Fail with MissingCompressibleExtension if CToken has extensions but no Compressible extension
      - Lazy load Clock sysvar for current_slot and Rent sysvar if not yet loaded (current_slot == 0)
      - Call `compressible.info.calculate_top_up_lamports(data_len, current_slot, lamports, rent_exemption)`
      - Subtract calculated top-up from lamports_budget

   d. **Validate budget:**
      - If no compressible accounts were found (current_slot == 0), exit early
      - If both top-up amounts are 0, exit early
      - If max_top_up != 0 and lamports_budget == 0, fail with MaxTopUpExceeded

   e. **Execute transfers:**
      - Call `multi_transfer_lamports(payer, &transfers)` to atomically transfer lamports
      - Updates account balances for both CMint and CToken if needed

**Errors:**

- `ProgramError::NotEnoughAccountKeys` (error code: 11) - Less than 3 accounts provided
- `ProgramError::InvalidInstructionData` (error code: 3) - Instruction data length is not 8 or 10 bytes
- `ProgramError::InsufficientFunds` (error code: 6) - Source CToken balance less than burn amount (from pinocchio burn), or payer has insufficient funds for top-up transfers
- `ProgramError::ArithmeticOverflow` (error code: 24) - Overflow when calculating total top-up amount
- Pinocchio token errors (converted to ProgramError::Custom):
  - `TokenError::OwnerMismatch` (error code: 4) - Authority is not owner or delegate
  - `TokenError::MintMismatch` (error code: 3) - CToken mint doesn't match CMint
  - `TokenError::AccountFrozen` (error code: 17) - CToken account is frozen
- `CTokenError::CMintDeserializationFailed` (error code: 18047) - Failed to deserialize CMint account using zero-copy
- `CTokenError::InvalidAccountData` (error code: 18002) - Account data length is too small, calculate top-up failed, or invalid account format
- `CTokenError::InvalidAccountState` (error code: 18036) - CToken account is not initialized (from zero-copy parsing)
- `CTokenError::InvalidAccountType` (error code: 18053) - Account is not a CToken account type (from zero-copy parsing)
- `CTokenError::SysvarAccessError` (error code: 18020) - Failed to get Clock or Rent sysvar for top-up calculation
- `CTokenError::MaxTopUpExceeded` (error code: 18043) - Total top-up amount (CMint + CToken) exceeds max_top_up limit
- `CTokenError::MissingCompressibleExtension` (error code: 18056) - CToken account has extensions but missing required Compressible extension

## Comparison with Token-2022

### Functional Parity

CToken Burn delegates core logic to `pinocchio_token_program::processor::burn::process_burn`, which implements SPL-compatible burn semantics:
- Balance/supply updates, authority validation, frozen check, mint mismatch check, delegate handling
- **BurnChecked:** CToken implements CTokenBurnChecked (discriminator: 15) with full decimals validation. See `CTOKEN_BURN_CHECKED.md`.

### CToken-Specific Features

**1. Compressible Top-Up Logic**
Automatically tops up CMint and CToken with rent lamports after burning to prevent accounts from becoming compressible.

**2. max_top_up Parameter**
10-byte instruction format adds `max_top_up` (u16) to limit combined top-up costs. Fails with `MaxTopUpExceeded` (18043) if exceeded.

### Unsupported SPL & Token-2022 Features

**1. No Multisig Support**
**2. No CPI Guard Extension Check**
**3. No Memo Transfer Extension Check**
**4. No Confidential Transfer Extension Check**

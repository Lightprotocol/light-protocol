## CToken BurnChecked

**discriminator:** 15
**enum:** `InstructionType::CTokenBurnChecked`
**path:** programs/compressed-token/program/src/ctoken/burn.rs

**description:**
Burns tokens from a decompressed CToken account and decreases the CMint supply with decimals validation, fully compatible with SPL Token BurnChecked semantics. Account layout `CToken` is defined in `program-libs/token-interface/src/state/ctoken/ctoken_struct.rs`. Account layout `CompressedMint` (CMint) is defined in `program-libs/token-interface/src/state/mint/compressed_mint.rs`. Extension layout `CompressionInfo` is defined in `program-libs/compressible/src/compression_info.rs` and is embedded in both CToken and CMint structs. Uses pinocchio-token-program to process the burn_checked (handles balance/supply updates, authority check, frozen check, decimals validation). After the burn, automatically tops up compressible accounts with additional lamports if needed. Top-up prevents accounts from becoming compressible during normal operations. Enforces max_top_up limit if provided (transaction fails if exceeded). Account order is REVERSED from mint_to instruction: [source_ctoken, cmint, authority] vs mint_to's [cmint, destination_ctoken, authority].

**Instruction data:**

Format 1 (9 bytes, legacy):
- Bytes 0-7: `amount` (u64, little-endian) - Number of tokens to burn
- Byte 8: `decimals` (u8) - Expected token decimals
- No max_top_up enforcement (effectively unlimited)

Format 2 (11 bytes):
- Bytes 0-7: `amount` (u64, little-endian) - Number of tokens to burn
- Byte 8: `decimals` (u8) - Expected token decimals
- Bytes 9-10: `max_top_up` (u16, little-endian) - Maximum lamports for combined CMint + CToken top-ups in units of 1,000 lamports (e.g., max_top_up=1 means 1,000 lamports, max_top_up=65535 means ~65.5M lamports). u16::MAX = no limit, 0 = no top-ups allowed.

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
   - Validated: decimals field matches instruction data decimals
   - Supply is decreased by burn amount
   - May receive rent top-up if compressible

3. authority
   - (signer)
   - Owner of the source CToken account
   - Must sign the transaction
   - If no fee_payer provided: also serves as payer for top-ups (must be writable)
   - If fee_payer provided: readonly (only needs to sign)

4. fee_payer (optional)
   - (signer, writable)
   - Optional separate account to pay for rent top-ups
   - If not provided, authority account pays for top-ups
   - Must have sufficient lamports to cover the top-up amount

**Instruction Logic and Checks:**

1. **Validate minimum accounts:**
   - Require at least 3 accounts (source CToken, CMint, authority/payer)
   - Return NotEnoughAccountKeys if insufficient

2. **Parse instruction data:**
   - Require at least 9 bytes (amount + decimals)
   - Parse max_top_up:
     - If instruction_data.len() == 9: max_top_up = u16::MAX (no limit, legacy format)
     - If instruction_data.len() == 11: parse u16 from bytes 9-10 as max_top_up
     - Otherwise: return InvalidInstructionData

3. **Process SPL burn_checked via pinocchio-token-program:**
   - Call `process_burn_checked` with first 9 bytes (amount + decimals)
   - Validates authority signature matches source CToken owner
   - Validates decimals match CMint's decimals field
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
      - Call `cmint_top_up_lamports_from_account_info(cmint, current_slot, program_id)`
      - Verifies CMint is owned by the expected program
      - Checks data length >= 262 bytes (minimum for CMint with CompressionInfo)
      - Validates account_type byte at offset 165 is ACCOUNT_TYPE_MINT
      - Reads CompressionInfo directly from bytes using `CompressionInfo::zero_copy_at`
      - Lazy loads Clock sysvar for current_slot if needed
      - Calls `calculate_top_up_lamports(data_len, current_slot, lamports)`
      - Returns None (skip) if any validation fails
      - Subtracts calculated top-up from lamports_budget

   c. **Calculate CToken top-up:**
      - Call `top_up_lamports_from_account_info_unchecked(ctoken, current_slot)`
      - Returns None (skip) if CToken data length < 272 bytes (minimum for Compressible extension)
      - Validates account_type byte at offset 165 is ACCOUNT_TYPE_TOKEN_ACCOUNT
      - Validates Option discriminator at offset 166 is Some
      - Validates first extension discriminator at offset 171 is Compressible
      - Reads CompressionInfo directly via bytemuck from bytes at offset 176
      - Lazy loads Clock sysvar for current_slot if needed
      - Calls `calculate_top_up_lamports(data_len, current_slot, lamports)`
      - Returns None (skip) if any validation fails
      - Subtracts calculated top-up from lamports_budget

   d. **Validate budget:**
      - If no compressible accounts were found (current_slot == 0), exit early
      - If both top-up amounts are 0, exit early
      - If max_top_up != u16::MAX and lamports_budget == 0, fail with MaxTopUpExceeded
      - If payer is None but top-up is needed, fail with MissingPayer

   e. **Execute transfers:**
      - Call `multi_transfer_lamports(payer, &transfers)` to atomically transfer lamports
      - Updates account balances for both CMint and CToken if needed

**Errors:**

- `ProgramError::NotEnoughAccountKeys` (error code: 11) - Less than 3 accounts provided
- `ProgramError::InvalidInstructionData` (error code: 3) - Instruction data length is not 9 or 11 bytes
- `ProgramError::InsufficientFunds` (error code: 6) - Source CToken balance less than burn amount (from pinocchio burn), or payer has insufficient funds for top-up transfers
- Pinocchio token errors (converted to ErrorCode variants via convert_pinocchio_token_error):
  - `ErrorCode::OwnerMismatch` - Authority is not owner or delegate
  - `ErrorCode::MintMismatch` - CToken mint doesn't match CMint
  - `ErrorCode::MintDecimalsMismatch` - Decimals don't match CMint's decimals
  - `ErrorCode::AccountFrozen` - CToken account is frozen
- `CTokenError::MaxTopUpExceeded` (error code: 18043) - Total top-up amount (CMint + CToken) exceeds max_top_up limit
- `CTokenError::MissingPayer` (error code: 18061) - Payer account required for top-up but not provided

## Comparison with Token-2022

### Functional Parity

CToken BurnChecked implements similar core functionality to SPL Token-2022's BurnChecked instruction:

1. **Balance/Supply Updates**: Decrease token account balance and mint supply by burn amount
2. **Authority Validation**: Verify owner signature or delegate authority
3. **Account State Checks**:
   - Frozen account check (fails if account is frozen)
   - Mint mismatch validation (account must belong to specified mint)
   - Insufficient funds check (account must have sufficient balance)
4. **Decimals Validation**: Validate instruction decimals match mint decimals
5. **Delegate Handling**: Support for burning via delegate with delegated amount tracking
6. **Permanent Delegate**: Honor permanent delegate authority if configured on mint

### CToken-Specific Features

1. **Compressible Top-Up Logic**: After burning, automatically tops up compressible accounts with rent lamports
2. **max_top_up Parameter**: Limits combined lamports spent on CMint + CToken top-ups
3. **Backwards Compatible Instruction Data**: Supports 9-byte (legacy) and 11-byte (with max_top_up) formats

### Missing Features

1. **No Multisig Support**: Only supports single-signer authority
2. **No PausableConfig Check**: Token-2022 fails if mint is paused
3. **No CpiGuard Check**: Token-2022 blocks burn in CPI context if guard enabled and authority is owner

### Instruction Data Comparison

| Token-2022 BurnChecked | CToken BurnChecked |
|------------------------|-------------------|
| 10 bytes (discriminator + amount + decimals) | 9 or 11 bytes (amount + decimals + optional max_top_up) |

### Account Layout Comparison

| Token-2022 BurnChecked | CToken BurnChecked |
|------------------------|-------------------|
| [source, mint, authority, ...signers] | [source_ctoken, cmint, authority] |
| 3+ accounts (for multisig) | Exactly 3 accounts |

### Security Notes

1. **Account Order Reversed from MintTo:**
   - CToken MintTo: [cmint, destination_ctoken, authority]
   - CToken BurnChecked: [source_ctoken, cmint, authority]

2. **Top-Up Payer is Authority:**
   - Authority (signer) serves as payer for rent top-ups
   - Burning tokens may require additional lamports from the authority's account

3. **Decimals Validation:**
   - Pinocchio validates instruction decimals against CMint's decimals field
   - Returns ErrorCode::MintDecimalsMismatch on mismatch

### Security Properties

**Shared:**
- Authority signature validation before state changes
- Account ownership by token program validation
- Overflow prevention in balance/supply arithmetic
- Frozen account protection
- Decimals mismatch protection

**CToken-Specific:**
- Authority lamport drainage protection via max_top_up
- Top-up atomicity: if top-up fails, entire instruction fails
- Compressibility timing management

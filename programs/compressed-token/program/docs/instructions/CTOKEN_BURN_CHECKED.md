## CToken BurnChecked

**discriminator:** 15
**enum:** `InstructionType::CTokenBurnChecked`
**path:** programs/compressed-token/program/src/ctoken_burn.rs

**description:**
Burns tokens from a decompressed CToken account and decreases the CMint supply with decimals validation, fully compatible with SPL Token BurnChecked semantics. Account layout `CToken` is defined in `program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs`. Account layout `CompressedMint` (CMint) is defined in `program-libs/ctoken-interface/src/state/mint/compressed_mint.rs`. Extension layout `CompressionInfo` is defined in `program-libs/compressible/src/compression_info.rs` and is embedded in both CToken and CMint structs. Uses pinocchio-token-program to process the burn_checked (handles balance/supply updates, authority check, frozen check, decimals validation). After the burn, automatically tops up compressible accounts with additional lamports if needed. Top-up prevents accounts from becoming compressible during normal operations. Enforces max_top_up limit if provided (transaction fails if exceeded). Account order is REVERSED from mint_to instruction: [source_ctoken, cmint, authority] vs mint_to's [cmint, destination_ctoken, authority].

**Instruction data:**

Format 1 (9 bytes, legacy):
- Bytes 0-7: `amount` (u64, little-endian) - Number of tokens to burn
- Byte 8: `decimals` (u8) - Expected token decimals
- No max_top_up enforcement (effectively unlimited)

Format 2 (11 bytes):
- Bytes 0-7: `amount` (u64, little-endian) - Number of tokens to burn
- Byte 8: `decimals` (u8) - Expected token decimals
- Bytes 9-10: `max_top_up` (u16, little-endian) - Maximum lamports for combined CMint + CToken top-ups (0 = no limit)

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
   - Also serves as payer for rent top-ups if needed

**Instruction Logic and Checks:**

1. **Validate minimum accounts:**
   - Require at least 3 accounts (source CToken, CMint, authority/payer)
   - Return NotEnoughAccountKeys if insufficient

2. **Parse instruction data:**
   - Require at least 9 bytes (amount + decimals)
   - Parse max_top_up:
     - If instruction_data.len() == 9: max_top_up = 0 (no limit, legacy format)
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
      - Borrow CMint data and deserialize using `CompressedMint::zero_copy_at`
      - Access compression info directly from mint.base.compression
      - Lazy load Clock sysvar for current_slot and Rent sysvar if not yet loaded
      - Call `compression.calculate_top_up_lamports(data_len, current_slot, lamports, rent_exemption)`
      - Subtract calculated top-up from lamports_budget

   c. **Calculate CToken top-up:**
      - Skip if CToken data length is 165 bytes (no extensions, standard SPL token account)
      - Borrow CToken data and deserialize using `CToken::zero_copy_at_checked`
      - Get Compressible extension via `token.get_compressible_extension()`
      - Fail with MissingCompressibleExtension if CToken has extensions but no Compressible extension
      - Lazy load Clock sysvar for current_slot and Rent sysvar if not yet loaded
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
- `ProgramError::InvalidInstructionData` (error code: 3) - Instruction data length is not 9 or 11 bytes
- `ProgramError::InsufficientFunds` (error code: 6) - Source CToken balance less than burn amount (from pinocchio burn), or payer has insufficient funds for top-up transfers
- `ProgramError::ArithmeticOverflow` (error code: 24) - Overflow when calculating total top-up amount
- Pinocchio token errors (converted to ProgramError::Custom):
  - `TokenError::OwnerMismatch` (error code: 4) - Authority is not owner or delegate
  - `TokenError::MintMismatch` (error code: 3) - CToken mint doesn't match CMint
  - `TokenError::MintDecimalsMismatch` (error code: 18) - Decimals don't match CMint's decimals
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
   - Returns MintDecimalsMismatch (error code: 18) on mismatch

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

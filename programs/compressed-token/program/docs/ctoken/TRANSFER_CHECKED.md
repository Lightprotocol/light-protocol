## CToken TransferChecked

**discriminator:** 12
**enum:** `InstructionType::CTokenTransferChecked`
**path:** programs/compressed-token/program/src/ctoken/transfer/checked.rs

### SPL Instruction Format Compatibility

This instruction uses the same account layout as SPL Token TransferChecked (source, mint, destination, authority) but has extended instruction data format.

When accounts require rent top-up, lamports are transferred directly from the authority account to the token accounts. The authority must have sufficient lamports to cover the top-up amount.

**Compatibility scenarios:**
- **SPL-compatible:** When using 9-byte instruction data (amount + decimals) with no top-up needed
- **Extended format:** When using 11-byte instruction data (amount + decimals + max_top_up) for compressible accounts

**Hot path optimization:**
When both source and destination accounts are exactly 165 bytes (no extensions), the instruction bypasses all extension processing and directly calls pinocchio process_transfer_checked for maximum performance.

**description:**
Transfers tokens between decompressed ctoken solana accounts with mint decimals validation, fully compatible with SPL Token TransferChecked semantics. Account layout `CToken` is defined in program-libs/token-interface/src/state/ctoken/ctoken_struct.rs. Compression info for rent top-up is defined in program-libs/compressible/src/compression_info.rs. Uses pinocchio-token-program to process the transfer (lightweight SPL-compatible implementation). After the transfer, automatically tops up compressible accounts with additional lamports if needed based on current slot and account balance. Top-up prevents accounts from becoming compressible during normal operations. Supports standard SPL Token transfer features including delegate authority and permanent delegate (multisig not supported). The transfer amount, authority validation, and decimals validation follow SPL Token TransferChecked rules exactly. Validates that mint decimals match the provided decimals parameter. Difference from CTokenTransfer: Requires mint account (4 accounts vs 3) for decimals validation and T22 extension validation.

**Instruction data:**
- **9 bytes (legacy):** amount (u64) + decimals (u8)
- **11 bytes (with max_top_up):** amount (u64) + decimals (u8) + max_top_up (u16)
  - max_top_up: Maximum lamports for top-up operations (0 = no limit)

**Accounts:**
1. source
   - (mutable)
   - The source ctoken account
   - Must have sufficient balance for the transfer
   - Must have same mint as destination
   - May receive rent top-up if compressible
   - If has cached decimals in compressible extension, used for validation

2. mint
   - (immutable)
   - The mint account for the token being transferred
   - Must match source and destination account mints
   - Decimals field must match instruction data decimals parameter
   - Required for T22 extension validation when accounts have restricted extensions

3. destination
   - (mutable)
   - The destination ctoken account
   - Must have same mint as source
   - Must have matching T22 extension markers (pausable, permanent_delegate, transfer_fee, transfer_hook)
   - May receive rent top-up if compressible

4. authority
   - (signer)
   - Owner of the source account or delegate with sufficient allowance
   - Must sign the transaction
   - If is permanent delegate, validated as signer and pinocchio validation is skipped
   - Also serves as payer for top-ups when accounts have compressible extension

**Instruction Logic and Checks:**

1. **Validate minimum accounts:**
   - Require at least 4 accounts (source, mint, destination, authority)
   - Return NotEnoughAccountKeys if insufficient

2. **Hot path for accounts without extensions:**
   - If both source and destination are exactly 165 bytes (no extensions):
     - Directly call pinocchio process_transfer_checked with first 9 bytes of instruction data
     - Skip all extension processing for maximum performance
     - Return immediately

3. **Validate instruction data:**
   - Must be at least 9 bytes (amount + decimals)
   - If 11 bytes, parse max_top_up from bytes [9..11]
   - If 9 bytes, set max_top_up = 0 (legacy, no limit)
   - Any other length returns InvalidInstructionData

4. **Parse max_top_up parameter:**
   - 0 = no limit on top-up lamports
   - Non-zero = maximum combined lamports for source + destination top-up
   - Transaction fails if calculated top-up exceeds max_top_up

5. **Process transfer extensions:**
   - Call process_transfer_extensions_transfer_checked from shared.rs with source, destination, authority, mint, and max_top_up
   - Validate sender (source account):
     - Deserialize source account (CToken) and extract extension information
     - Validate mint account matches source token's mint field
     - Check for T22 restricted extensions (pausable, permanent_delegate, transfer_fee, transfer_hook, default_account_state)
     - If source has restricted extensions, deserialize and validate mint extensions once:
       - Mint must not be paused
       - Transfer fees must be zero
       - Transfer hooks must have nil program_id
       - Extract permanent delegate if present
     - Validate permanent delegate authority if applicable
     - Cache decimals from compressible extension if has_decimals flag is set
   - Validate recipient (destination account):
     - Deserialize destination account and extract extension information
     - No mint validation for recipient (only sender needs to match mint)
     - Extract T22 extension markers
   - Verify sender and destination have matching T22 extension markers
   - Calculate top-up amounts for both accounts based on compression info:
     - Get current slot from Clock sysvar (lazy loaded once)
     - Call calculate_top_up_lamports for each account
   - Transfer lamports from authority to accounts if top-up needed:
     - Check max_top_up budget if set (non-zero)
     - Execute multi_transfer_lamports atomically
   - Return (signer_is_validated, extension_decimals) tuple

6. **Extract decimals and execute transfer:**
   - Parse amount and decimals from instruction data using unpack_amount_and_decimals
   - If source account has cached decimals in compressible extension (extension_decimals is Some):
     - Validate extension_decimals == instruction decimals parameter
     - Create accounts slice without mint: [source, destination, authority]
     - Call pinocchio process_transfer with expected_decimals = None (3 accounts)
     - signer_is_validated flag from permanent delegate check skips redundant owner/delegate validation
   - If no cached decimals (extension_decimals is None):
     - Validate mint account owner is token program
     - Call pinocchio process_transfer with all 4 accounts [source, mint, destination, authority] and expected_decimals = Some(decimals)
     - signer_is_validated flag from permanent delegate check skips redundant owner/delegate validation
   - pinocchio-token-program validates:
     - Source and destination have same mint
     - Mint decimals match provided decimals parameter (when expected_decimals is Some)
     - Authority is owner or delegate with sufficient allowance (unless signer_is_validated is true)
     - Source has sufficient balance
     - Accounts are not frozen
     - Delegate amount is decremented if delegated transfer
   - Transfers amount from source to destination

**Errors:**

- `ProgramError::NotEnoughAccountKeys` (error code: 11) - Less than 4 accounts provided
- `ProgramError::InvalidInstructionData` (error code: 3) - Instruction data is not 9 or 11 bytes, or decimals validation failed
- `ProgramError::MissingRequiredSignature` (error code: 8) - Authority is permanent delegate but not a signer
- `CTokenError::InvalidAccountData` (error code: 18002) - Failed to deserialize CToken account, mint mismatch, or invalid extension data
- `CTokenError::SysvarAccessError` (error code: 18020) - Failed to get Clock sysvar for top-up calculation
- `CTokenError::MaxTopUpExceeded` (error code: 18043) - Calculated top-up exceeds max_top_up limit
- `ProgramError::InsufficientFunds` (error code: 6) - Source balance less than amount (pinocchio error)
- Pinocchio token errors (converted to ProgramError::Custom):
  - `TokenError::OwnerMismatch` (error code: 4) - Authority is not owner or delegate
  - `TokenError::MintMismatch` (error code: 3) - Source and destination have different mints or mint account mismatch
  - `TokenError::AccountFrozen` (error code: 17) - Source or destination account is frozen
  - `TokenError::InsufficientFunds` (error code: 1) - Delegate has insufficient allowance
  - `TokenError::InvalidMint` (error code: 2) - Mint decimals do not match provided decimals parameter
- `ErrorCode::MintRequiredForTransfer` (error code: 6128) - Account has restricted extensions but mint account not provided
- `ErrorCode::MintPaused` (error code: 6127) - Mint has pausable extension and is currently paused
- `ErrorCode::NonZeroTransferFeeNotSupported` (error code: 6129) - Mint has non-zero transfer fee configured
- `ErrorCode::TransferHookNotSupported` (error code: 6130) - Mint has transfer hook with non-nil program_id

## Comparison with SPL Token

### Functional Parity

CToken delegates core logic to `pinocchio_token_program::processor::shared::transfer::process_transfer`, which implements SPL Token-compatible transfer semantics. When `expected_decimals` is Some, it performs decimals validation against the mint account:
- Authority validation, balance updates, frozen check, mint matching, decimals validation (when expected_decimals is Some)

Note: For the hot path (165-byte accounts without extensions), `pinocchio_token_program::processor::transfer_checked::process_transfer_checked` is called directly.

### CToken-Specific Features

**1. Compressible Top-Up Logic**
Automatically tops up source and destination accounts with rent lamports after transfer to prevent accounts from becoming compressible.

**2. max_top_up Parameter**
11-byte instruction format adds `max_top_up` (u16) to limit combined top-up costs. Fails with `MaxTopUpExceeded` (18043) if exceeded.

**3. Cached Decimals Optimization**
If source CToken has cached decimals in Compressible extension, validates against instruction and can skip mint account read.

### Unsupported SPL & Token-2022 Features

**1. No Multisig Support**
**2. No CPI Guard Extension Check**
**3. No Memo Transfer Extension Check**
**4. No Confidential Transfer Extension Check**
**5. No NonTransferable Extension Check**
**6. No Native SOL Support**
**7. No TransferFee Handling** - Rejects mints with non-zero transfer fees
**8. No TransferHook Execution** - Rejects mints with non-nil hook program_id

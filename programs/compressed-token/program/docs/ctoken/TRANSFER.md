## CToken Transfer

**discriminator:** 3
**enum:** `InstructionType::CTokenTransfer`
**path:** programs/compressed-token/program/src/ctoken/transfer/default.rs

### SPL Instruction Format Compatibility

**Important:** This instruction uses the same account layout as SPL Token transfer (source, destination, authority) but has extended instruction data format.

When accounts require rent top-up, lamports are transferred directly from the authority account to the token accounts. The authority must have sufficient lamports to cover the top-up amount.

**Compatibility scenarios:**
- **SPL-compatible:** When using 8-byte instruction data (amount only) with no top-up needed
- **Extended format:** When using 10-byte instruction data (amount + max_top_up) for compressible accounts

**description:**
1. Transfers tokens between decompressed ctoken solana accounts, fully compatible with SPL Token semantics
2. Account layout `CToken` is defined in path: program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs
3. Compression info for rent top-up is defined in: program-libs/compressible/src/compression_info.rs
4. Uses pinocchio-token-program to process the transfer (lightweight SPL-compatible implementation)
5. After the transfer, automatically tops up compressible accounts with additional lamports if needed:
   - Calculates top-up requirements based on current slot and account balance
   - Only applies to accounts with compression info in their base state
   - Top-up prevents accounts from becoming compressible during normal operations
6. Supports standard SPL Token transfer features including delegate authority and permanent delegate (multisig not supported)
7. The transfer amount and authority validation follow SPL Token rules exactly
8. Validates T22 extension markers match between source and destination (pausable, permanent_delegate, transfer_fee, transfer_hook, default_account_state)

**Instruction data:**
After discriminator byte, the following formats are supported:
- **8 bytes (legacy):** amount (u64) - No max_top_up enforcement
- **10 bytes (extended):** amount (u64) + max_top_up (u16)
  - `amount`: u64 - Number of tokens to transfer
  - `max_top_up`: u16 - Maximum lamports for top-up (0 = no limit)

**Accounts:**
1. source
   - (mutable)
   - The source ctoken account
   - Must have sufficient balance for the transfer
   - May receive rent top-up if compressible

2. destination
   - (mutable)
   - The destination ctoken account
   - Must have same mint as source
   - May receive rent top-up if compressible

3. authority
   - (signer)
   - Owner of the source account or delegate with sufficient allowance
   - Must sign the transaction

Note: The authority account (index 2) also serves as the payer for top-ups when accounts have compressible extension.

**Instruction Logic and Checks:**

1. **Validate minimum accounts:**
   - Require at least 3 accounts (source, destination, authority)
   - Return NotEnoughAccountKeys if insufficient

2. **Validate minimum instruction data:**
   - Must be at least 8 bytes (amount)
   - Return InvalidInstructionData if less than 8 bytes

3. **Hot path optimization (no extensions):**
   - If both source and destination accounts are exactly 165 bytes (standard SPL token account size without extensions):
     - Skip all extension processing and max_top_up validation
     - Pass only the first 8 bytes (amount) directly to pinocchio SPL transfer
     - This is the fast path for accounts without compressible or T22 extensions

4. **Parse max_top_up (extended path only):**
   - If 10 bytes, parse max_top_up from bytes [8..10]
   - If 8 bytes, set max_top_up = 0 (legacy, no limit)
   - Any other length returns InvalidInstructionData

5. **Process transfer extensions:**
   - Call `process_transfer_extensions` from shared.rs with source, destination, authority (no mint)

   a. **Validate sender (source account):**
      - Deserialize source account (CToken) using zero-copy
      - Check for T22 restricted extensions (pausable, permanent_delegate, transfer_fee, transfer_hook, default_account_state)
      - If source has restricted extension markers:
        - Require mint account (MintRequiredForTransfer error if not provided)
        - Fail with MintHasRestrictedExtensions if mint has any restricted extensions
        - Note: CTokenTransfer does NOT support restricted extensions; use CTokenTransferChecked instead
      - Validate permanent delegate authority if applicable (from mint extension)
      - Calculate top-up lamports from compression info

   b. **Validate recipient (destination account):**
      - Deserialize destination account and extract extension information
      - Extract T22 extension markers
      - Calculate top-up lamports from compression info

   c. **Check T22 extension consistency:**
      - Verify sender and destination have matching T22 extension markers
      - Error if flags mismatch (InvalidInstructionData)

   d. **Perform compressible top-up:**
      - Check max_top_up budget if set (non-zero)
      - Execute multi_transfer_lamports from authority to accounts

6. **Process SPL transfer:**
   - Call pinocchio_token_program::processor::transfer::process_transfer
   - Pass only the first 8 bytes (amount) to the processor
   - Pass signer_is_validated flag if permanent delegate was validated

**Errors:**

- `ProgramError::NotEnoughAccountKeys` (error code: 11) - Less than 3 accounts provided
- `ProgramError::InvalidInstructionData` (error code: 3) - Instruction data is not 8 or 10 bytes, or T22 extension flags mismatch between source and destination
- `ProgramError::MissingRequiredSignature` (error code: 8) - Authority is permanent delegate but not a signer
- `ProgramError::InsufficientFunds` (error code: 6) - Source balance less than amount (pinocchio error)
- Pinocchio token errors (converted to ProgramError::Custom):
  - `TokenError::OwnerMismatch` (error code: 4) - Authority is not owner or delegate
  - `TokenError::MintMismatch` (error code: 3) - Source and destination have different mints
  - `TokenError::AccountFrozen` (error code: 17) - Source or destination account is frozen
  - `TokenError::InsufficientFunds` (error code: 1) - Delegate has insufficient allowance
- `CTokenError::InvalidAccountData` (error code: 18002) - Failed to deserialize CToken account, mint mismatch, or invalid extension data
- `CTokenError::SysvarAccessError` (error code: 18020) - Failed to get Clock sysvar for top-up calculation
- `CTokenError::MaxTopUpExceeded` (error code: 18043) - Calculated top-up exceeds max_top_up limit
- `ErrorCode::MintRequiredForTransfer` (error code: 6128) - Source account has restricted extension markers but mint account not provided
- `ErrorCode::MintHasRestrictedExtensions` (error code: 6142) - Mint has restricted extensions; CTokenTransfer does not support restricted extensions (use CTokenTransferChecked instead)

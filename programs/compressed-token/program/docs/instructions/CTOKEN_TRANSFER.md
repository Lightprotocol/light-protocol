## CToken Transfer

**discriminator:** 3
**enum:** `InstructionType::CTokenTransfer`
**path:** programs/compressed-token/program/src/ctoken_transfer.rs

**description:**
1. Transfers tokens between decompressed ctoken solana accounts, fully compatible with SPL Token semantics
2. Account layout `CToken` is defined in path: program-libs/ctoken-types/src/state/ctoken/ctoken_struct.rs
3. Extension layout `CompressionInfo` is defined in path: program-libs/ctoken-types/src/state/extensions/compressible.rs
4. Uses light_token_22 fork to process the transfer (required because token_22 has hardcoded program ID checks)
5. After the transfer, automatically tops up compressible accounts with additional lamports if needed:
   - Calculates top-up requirements based on current slot and account balance
   - Only applies to accounts with compressible extension
   - Top-up prevents accounts from becoming compressible during normal operations
6. Supports standard SPL Token transfer features including delegate authority (multisig not supported)
7. The transfer amount and authority validation follow SPL Token rules exactly

**Instruction data:**
- First byte: instruction discriminator (3)
- Second byte: 0 (padding)
- Remaining bytes: SPL TokenInstruction::Transfer serialized
  - `amount`: u64 - Number of tokens to transfer

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

4. payer (when accounts have compressible extension)
   - (signer, mutable)
   - Pays for rent top-ups if needed
   - Must be the third account if any account needs top-up

**Instruction Logic and Checks:**

1. **Parse instruction data:**

2. **Validate minimum accounts:**
   - Require at least 3 accounts (source, destination, authority/payer)
   - Return NotEnoughAccountKeys if insufficient

3. **Convert account formats:**
   - Convert Pinocchio AccountInfos to Anchor AccountInfos

4. **Process SPL transfer:**
   - Call light_token_22::Processor::process_transfer

5. **Calculate top-up requirements:**
   For each of source and destination accounts:

   a. **Check for compressible extension:**
      - Skip if account size is base size (no extensions)
      - Parse extensions if present
      - Error if extensions exist but no Compressible found

   b. **Calculate top-up amount:**
      - Get current slot from Clock sysvar (lazy loaded)
      - Call `calculate_top_up_lamports` which:
        - Checks if account is compressible
        - Calculates rent deficit if any
        - Adds configured lamports_per_write amount
        - Returns 0 if account is well-funded

6. **Execute top-up transfers:**
   - Skip if no accounts need top-up (current_slot == 0 indicates no compressible accounts)
   - Use payer account (third account) as funding source
   - Execute multi_transfer_lamports to top up both accounts atomically
   - Update account lamports balances

**Errors:**

- `ProgramError::NotEnoughAccountKeys` (error code: 11) - Less than 3 accounts provided
- `ProgramError::InvalidInstructionData` (error code: 3) - Instruction is not TokenInstruction::Transfer or failed to unpack instruction data
- `ProgramError::InsufficientFunds` (error code: 6) - Source balance less than amount (SPL Token error)
- `ProgramError::Custom` (SPL Token errors) - OwnerMismatch, MintMismatch, AccountFrozen, or InvalidDelegate from SPL token validation
- `CTokenError::InvalidAccountData` (error code: 18002) - Account has extensions but no Compressible extension or failed to parse extensions
- `CTokenError::SysvarAccessError` (error code: 18020) - Failed to get Clock sysvar for current slot

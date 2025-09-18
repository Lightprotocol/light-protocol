## Transfer2

**discriminator:** 104
**enum:** `CTokenInstruction::Transfer2`
**path:** programs/compressed-token/program/src/transfer2/

**description:**
1. Batch transfer instruction supporting multiple token operations in a single transaction with up to 5 different mints (cmints or spl)

2. Account types and data layouts:
   - Compressed accounts: `TokenData` (program-libs/ctoken-types/src/state/token_data.rs)
   - Decompressed Solana accounts: `CompressedToken` for ctokens (program-libs/ctoken-types/src/state/solana_ctoken.rs) or standard SPL token accounts
   - SPL tokens when compressed are backed by tokens stored in ctoken pool PDAs

3. Compression modes:
   - `Compress`: Move tokens from Solana account (ctoken or SPL) to compressed state
   - `Decompress`: Move tokens from compressed state to Solana account (ctoken or SPL)
   - `CompressAndClose`: Compress full ctoken balance and close the account (authority: owner or rent authority for compressible accounts)

4. Global sum check enforces transaction balance:
   - Input sum = compressed inputs + compress operations (tokens entering compressed state)
   - Output sum = compressed outputs + decompress operations (tokens leaving compressed state)
   - Each mint must balance to zero (input sum = output sum)
   - Enables implicit cross-type transfers (SPL↔ctoken) without creating compressed accounts

5. CPI context support for cross-program invocations:
   - Write mode: Only compressed-to-compressed transfers allowed (no Solana account modifications)
   - Execute mode: All operations supported including compress/decompress

**Instruction data:**
1. instruction data is defined in path: program-libs/ctoken-types/src/instructions/transfer2.rs
   - `with_transaction_hash`: Compute transaction hash for the complete transaction and include in compressed account data, enables ZK proofs over how compressed accounts are spent
   - `with_lamports_change_account_merkle_tree_index`: Track lamport changes in specified tree
   - `proof`: Optional CompressedProof - Required for ZK validation of compressed inputs; not needed for proof by index or when no compressed inputs exist
   - `in_token_data`: Vec<MultiInputTokenDataWithContext> - Input compressed token accounts (packed: owner/delegate/mint are indices to packed accounts) with merkle context (root index, tree/queue indices, leaf index, proof-by-index bool)
   - `out_token_data`: Vec<MultiTokenTransferOutputData> - Output compressed token accounts (packed: owner/delegate/mint/merkle_tree are indices to packed accounts)
   - `in_lamports`: Optional lamport amounts for input accounts (unimplemented)
   - `out_lamports`: Optional lamport amounts for output accounts (unimplemented)
   - `in_tlv`: Optional TLV data for input accounts (unimplemented)
   - `out_tlv`: Optional TLV data for output accounts (unimplemented)
   - `compressions`: Optional Vec<Compression> - Compress/decompress operations
   - `cpi_context`: Optional CompressedCpiContext - Required for CPI operations; write mode: set either first_set_context or set_context (not both); execute mode: provide with all flags false

2. Compression struct fields (path: program-libs/ctoken-types/src/instructions/transfer2.rs):
   - `mode`: CompressionMode enum (Compress, Decompress, CompressAndClose)
   - `amount`: u64 - Amount to compress/decompress
   - `mint`: u8 - Index of mint account in packed accounts
   - `source_or_recipient`: u8 - Index of source (compress) or recipient (decompress) account
   - `authority`: u8 - Index of owner/delegate account (compress only)
   - `pool_account_index`: u8 - For SPL: pool account index; For CompressAndClose: rent_sponsor_index
   - `pool_index`: u8 - For SPL: pool index; For CompressAndClose: compressed_account_index
   - `bump`: u8 - For SPL: pool PDA bump; For CompressAndClose: destination_index

**Accounts:**
1. light_system_program
   - non-mutable
   - Light Protocol system program for compressed account operations
   - Optional if no_compressed_accounts (only decompressed operations)

System accounts (when compressed accounts involved):
2. fee_payer
   - (signer, mutable)
   - Pays transaction fees and rent for new compressed accounts

3. authority
   - (signer)
   - Transaction authority for system operations

4. cpi_authority_pda
   - PDA signer for CPI calls to light system program
   - Seeds: [CPI_AUTHORITY_SEED]

5. registered_program_pda
   - Legacy account for program registration

6. account_compression_authority
   - Account compression authority PDA

7. account_compression_program
   - Merkle tree account compression program

8. system_program
   - System program for account operations

9. sol_pool_pda (optional)
   - (mutable)
   - Required when input_lamports != output_lamports
   - Handles lamport imbalances in compressed accounts

10. sol_decompression_recipient (optional)
    - (mutable)
    - Required when decompressing lamports (input_lamports < output_lamports)
    - Receives decompressed SOL

11. cpi_context_account (optional)
    - (mutable)
    - For storing CPI context data for later execution

Compressions-only accounts (when no_compressed_accounts):
12. compressions_only_cpi_authority_pda
    - PDA signer for compression operations
    - Seeds: [CPI_AUTHORITY_SEED]

13. compressions_only_fee_payer
    - (signer, mutable)
    - Pays for compression/decompression operations

Packed accounts (dynamic indexing):
- merkle tree and queue accounts - For compressed account storage, nullifier tracking and output storage (must come first, identified by ACCOUNT_COMPRESSION_PROGRAM ownership)
- mint accounts - Referenced by index in instruction data (account doesn't need to exist, only pubkey is used)
- owner accounts - Token account owners referenced by index
- delegate accounts - Optional delegates referenced by index
- token accounts - Decompressed ctoken or SPL token accounts for compress/decompress operations

**Instruction Logic and Checks:**

1. **Common initialization (all paths):**
   - Deserialize `CompressedTokenInstructionDataTransfer2` using zero-copy
   - Validate CPI context via `check_cpi_context`: Ensures `set_context || first_set_context` is false when `cpi_context` is Some
   - Validate instruction data via `validate_instruction_data`:
     - Check unimplemented features (`in_lamports`, `out_lamports`, `in_tlv`, `out_tlv`) are None
     - Ensure CPI context write mode (`set_context || first_set_context`) has no compressions
   - Determine required optional accounts via `Transfer2Config::from_instruction_data`:
     - Analyzes instruction data to identify which optional accounts must be present
     - Sets `sol_pool_required` when lamport imbalance exists (input ≠ output lamports)
     - Sets `sol_decompression_required` when decompressing SOL (input < output lamports)
     - Sets `cpi_context_required` when CPI context operations needed
     - Sets `no_compressed_accounts` when no compressed accounts involved (in_token_data and out_token_data both empty)
     - Uses checked arithmetic to prevent lamport calculation overflow
   - Validate and parse accounts via `Transfer2Accounts::validate_and_parse`

2. **Branch based on compressed account involvement:**

**Path A: No Compressed Accounts (compressions-only operations)**
   If `no_compressed_accounts` is true, execute `process_no_system_program_cpi`:

   a. **Validate compressions-only accounts:**
      - Extract `compressions_only_fee_payer` (error: CompressionsOnlyMissingFeePayer if missing)
      - Extract `compressions_only_cpi_authority_pda` (error: CompressionsOnlyMissingCpiAuthority if missing)
      - Validate compressions exist (error: NoInputsProvided if missing)

   b. **Process compression operations:**
      - Create mint sums tracker (ArrayVec with 5-mint limit)
      - Run `sum_compressions` to validate compression balance per mint:
        - For Decompress: verify existing balance (error: SumCheckFailed if no balance to decompress)
        - Check mint tracker capacity (error: TooManyMints if exceeds 5)
      - Execute `process_token_compression` for compress/decompress operations

   c. **Close accounts:**
      - Execute `close_for_compress_and_close` for each CompressAndClose operation:
        - Validates token account can be closed (authority is owner OR rent authority for compressible accounts)
        - Transfers rent to rent_sponsor, remaining lamports to destination
        - Zeros account data and resizes to 0 bytes

   d. **Exit without light-system-program CPI**

**Path B: With Compressed Accounts (full transfer operations)**
   If compressed accounts are involved, execute `process_with_system_program_cpi`:

   a. **Prepare CPI instruction:**
      - Allocate CPI instruction bytes via `allocate_cpi_bytes`
      - Create zero-copy CPI instruction struct via `InstructionDataInvokeCpiWithReadOnly::new_zero_copy`
      - Initialize CPI instruction with proof and context
      - Create `HashCache` for pubkey hash reuse (Poseidon optimization)

   b. **Process compressed accounts:**
      - Set input compressed accounts via `set_input_compressed_accounts`:
        - Hash token data (Poseidon for versions 1-2 with pubkeys pre-hashed to field size, SHA256 for version 3/ShaFlat)
        - Add merkle context and root indices
      - Set output compressed accounts via `set_output_compressed_accounts`:
        - Create new compressed accounts with updated balances
        - Hash token data and assign to appropriate merkle trees

   c. **Validate transaction balance:**
      - Run `sum_check_multi_mint` across all mints (up to 5 supported)
      - Track running sums per mint: compressed inputs + compress operations vs compressed outputs + decompress operations
      - Verify final sum is zero for each mint (perfect balance)

   d. **Execute based on system account type:**

      **System CPI Path:**
      If `validated_accounts.system` exists:
      - Execute `process_token_compression` for compress/decompress operations
      - Extract CPI accounts and tree pubkeys via `validated_accounts.cpi_accounts`
      - Execute `execute_cpi_invoke` with light-system-program
      - Execute `close_for_compress_and_close` for CompressAndClose operations

      **CPI Context Write Path:**
      If `validated_accounts.write_to_cpi_context_system` exists:
      - Validate exactly 4 accounts provided (error: Transfer2CpiContextWriteInvalidAccess if not)
      - Accounts: [0] light-system-program, [1] fee_payer, [2] cpi_authority_pda, [3] cpi_context
      - Execute `execute_cpi_invoke` in write-only mode (no tree accounts)
      - No SOL pool operations allowed (error: Transfer2CpiContextWriteWithSolPool)

**Compression/Decompression Processing Details:**

When `process_token_compression` is called (in both Path A and Path B):

1. **Main routing logic:**
   - Iterate through each compression in the compressions array
   - Get source_or_recipient account from packed accounts
   - Route to handler based on account owner:
     - ctoken program → native compression handler
     - SPL Token → SPL compression handler
     - SPL Token 2022 → SPL compression handler
     - Other → error (InvalidInstructionData)

2. **SPL Token compression/decompression:**
   - Validate compression mode fields (authority must be 0 for Decompress)
   - Get mint and token pool PDA from packed accounts
   - Validate pool PDA derivation matches [mint, pool_index] with provided bump
   - **For Compress:**
     - Get authority account from packed accounts
     - Transfer from user account to pool via SPL token CPI (authority must be signer, checked by SPL program)
   - **For Decompress:**
     - Transfer from pool to user account via SPL token CPI with PDA signer (CPI authority PDA signs)

3. **Native (ctoken) compression/decompression:**
   - Validate compression mode fields (authority must be 0 for Decompress)
   - Verify account owned by ctoken program
   - Deserialize account as CompressedToken
   - Verify mint matches between account and compression
   - **For Compress:**
     - Validate authority via `verify_and_update_token_account_authority`:
       - Check authority is signer (error: InvalidSigner)
       - If authority == owner: proceed
       - If authority == delegate: verify delegated amount ≥ compression amount, update delegation
       - Otherwise: error (OwnerMismatch)
     - Check sufficient balance (error: ArithmeticOverflow)
     - Subtract amount from balance with overflow protection
   - **For Decompress:**
     - Add amount to balance with overflow protection
   - Calculate compressible extension top-up if present
   - Execute lamport transfers (max 40, error: TooManyCompressionTransfers)

**Errors:**

- `ProgramError::BorshIoError` (error code: 15) - Failed to deserialize instruction data
- `ProgramError::NotEnoughAccountKeys` (error code: 11) - Missing required accounts
- `ProgramError::InvalidInstructionData` (error code: 3) - Invalid instruction data or authority index for decompress mode
- `ProgramError::InvalidAccountData` (error code: 4) - Account data deserialization fails
- `ProgramError::ArithmeticOverflow` (error code: 24) - Overflow in lamport calculations
- `CTokenError::TokenDataTlvUnimplemented` (error code: 18035) - TLV data not yet supported
- `CTokenError::CompressedTokenAccountTlvUnimplemented` (error code: 18021) - Compressed account TLV not supported
- `CTokenError::InvalidInstructionData` (error code: 18001) - Compressions not allowed when writing to CPI context
- `CTokenError::InvalidCompressionMode` (error code: 18018) - Invalid compression mode value
- `CTokenError::CompressInsufficientFunds` (error code: 18019) - Insufficient balance for compression
- `CTokenError::InsufficientSupply` (error code: 18010) - Insufficient token supply for operation
- `CTokenError::ArithmeticOverflow` (error code: 18003) - Arithmetic overflow in balance calculations
- `ErrorCode::SumCheckFailed` (error code: 6005) - Input/output token amounts don't match
- `ErrorCode::InputsOutOfOrder` (error code: 6054) - Sum inputs mint indices not in ascending order
- `ErrorCode::TooManyMints` (error code: 6055) - Sum check, too many mints (max 5)
- `ErrorCode::ComputeOutputSumFailed` (error code: 6002) - Output mint not in inputs or compressions
- `ErrorCode::TooManyCompressionTransfers` (error code: 6106) - Too many compression transfers. Maximum 40 transfers allowed per instruction
- `ErrorCode::NoInputsProvided` (error code: 6025) - No compressions provided in early exit path (no compressed accounts)
- `ErrorCode::CompressionsOnlyMissingFeePayer` (error code: 6026) - Missing fee payer for compressions-only operations
- `ErrorCode::CompressionsOnlyMissingCpiAuthority` (error code: 6027) - Missing CPI authority PDA for compressions-only operations
- `ErrorCode::OwnerMismatch` (error code: 6075) - Authority doesn't match account owner or delegate
- `ErrorCode::Transfer2CpiContextWriteInvalidAccess` (error code: 6082) - Invalid access to system accounts during CPI write
- `ErrorCode::Transfer2CpiContextWriteWithSolPool` (error code: 6083) - SOL pool operations not supported with CPI context write
- `ErrorCode::Transfer2InvalidChangeAccountData` (error code: 6084) - Change account contains unexpected token data
- `ErrorCode::CpiContextExpected` (error code: 6085) - CPI context required but not provided
- `ErrorCode::CompressAndCloseDestinationMissing` (error code: 6087) - Missing destination for CompressAndClose
- `ErrorCode::CompressAndCloseAuthorityMissing` (error code: 6088) - Missing authority for CompressAndClose
- `ErrorCode::CompressAndCloseAmountMismatch` (error code: 6090) - CompressAndClose amount doesn't match balance
- `ErrorCode::CompressAndCloseDelegateNotAllowed` (error code: 6092) - Delegates cannot use CompressAndClose
- `AccountError::InvalidSigner` (error code: 12015) - Required signer account is not signing
- `AccountError::AccountNotMutable` (error code: 12008) - Required mutable account is not mutable
- Additional errors from close_token_account for CompressAndClose operations

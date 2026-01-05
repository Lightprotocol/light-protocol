## Transfer2

### Navigation

| I want to... | Go to |
|-------------|-------|
| Transfer compressed tokens | → [Path B](#path-b-with-compressed-accounts-full-transfer-operations) (line 161) + [System accounts](#system-accounts-when-compressed-accounts-involved) (line 60) |
| Only compress/decompress (no transfers) | → [Path A](#path-a-no-compressed-accounts-compressions-only-operations) (line 134) + [Compressions-only accounts](#compressions-only-accounts-when-no_compressed_accounts) (line 99) |
| Compress SPL tokens | → [SPL compression](#spl-token-compressiondecompression) (line 217) |
| Compress CToken accounts | → [CToken compression](#ctoken-compressiondecompression-srctransfer2compressionctoken) (line 227) |
| Close compressible account (forester) | → [CompressAndClose](#for-compressandclose) (line 243) - compression_authority only |
| Use CPI context | → [Write mode](#cpi-context-write-path) (line 192) or [Execute mode](#cpi-context-support-for-cross-program-invocations) (line 27) |
| Debug errors | → [Error reference](#errors) (line 275) |

**discriminator:** 101
**enum:** `InstructionType::Transfer2`
**path:** programs/compressed-token/program/src/transfer2/

**description:**
1. Batch transfer instruction supporting multiple token operations in a single transaction with up to 5 different mints (cmints or spl)

2. Account types and data layouts:
   - Compressed accounts: `TokenData` (program-libs/ctoken-interface/src/state/compressed_token/token_data.rs)
   - Decompressed Solana accounts: `CToken` for ctokens (program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs) or standard SPL token accounts
   - SPL tokens when compressed are backed by tokens stored in ctoken pool PDAs

3. Compression modes:
   - `Compress`: Move tokens from Solana account (ctoken or SPL) to compressed state
   - `Decompress`: Move tokens from compressed state to Solana account (ctoken or SPL)
   - `CompressAndClose`: Compress full ctoken balance and close the account (authority: compression_authority only, requires compressible extension, **ctoken accounts only - NOT supported for SPL tokens**)

4. Global sum check enforces transaction balance:
   - Input sum = compressed inputs + compress operations (tokens entering compressed state)
   - Output sum = compressed outputs + decompress operations (tokens leaving compressed state)
   - Each mint must balance to zero (input sum = output sum)
   - Enables implicit cross-type transfers (SPL↔ctoken) without creating compressed accounts

5. CPI context support for cross-program invocations:
   - Write mode: Only compressed-to-compressed transfers allowed (no Solana account modifications)
   - Execute mode: All operations supported including compress/decompress

**Instruction data:**
1. instruction data is defined in path: program-libs/ctoken-interface/src/instructions/transfer2/instruction_data.rs
   - `with_transaction_hash`: Compute transaction hash for the complete transaction and include in compressed account data, enables ZK proofs over how compressed accounts are spent
   - `with_lamports_change_account_merkle_tree_index`: Track lamport changes in specified tree
   - `proof`: Optional CompressedProof - Required for ZK validation of compressed inputs; not needed for proof by index or when no compressed inputs exist
   - `in_token_data`: Vec<MultiInputTokenDataWithContext> - Input compressed token accounts (packed: owner/delegate/mint are indices to packed accounts) with merkle context (root index, tree/queue indices, leaf index, proof-by-index bool)
   - `out_token_data`: Vec<MultiTokenTransferOutputData> - Output compressed token accounts (packed: owner/delegate/mint/merkle_tree are indices to packed accounts)
   - `in_lamports`: Optional lamport amounts for input accounts (unimplemented)
   - `out_lamports`: Optional lamport amounts for output accounts (unimplemented)
   - `in_tlv`: Optional TLV data for input accounts (used for CompressedOnly extension during decompress)
   - `out_tlv`: Optional TLV data for output accounts (used for CompressedOnly extension during CompressAndClose)
   - `compressions`: Optional Vec<Compression> - Compress/decompress operations
   - `cpi_context`: Optional CompressedCpiContext - Required for CPI operations; write mode: set either first_set_context or set_context (not both); execute mode: provide with all flags false

2. Compression struct fields (path: program-libs/ctoken-interface/src/instructions/transfer2/compression.rs):
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
     - Check unimplemented features (`in_lamports`, `out_lamports`) are None
     - Validate `in_tlv` length matches `in_token_data` length if provided
     - Validate `out_tlv` length matches `out_token_data` length if provided
     - Block CompressedOnly inputs from having compressed outputs (error: CompressedOnlyBlocksTransfer)
     - Ensure CPI context write mode (`set_context || first_set_context`) has no compressions
   - Determine required optional accounts via `Transfer2Config::from_instruction_data`:
     - Analyzes instruction data to identify which optional accounts must be present
     - Sets `sol_pool_required` when lamport imbalance exists (input ≠ output lamports)
     - Sets `sol_decompression_required` when decompressing SOL (input < output lamports)
     - Sets `cpi_context_required` when CPI context operations needed
     - Sets `no_compressed_accounts` when no compressed accounts involved (in_token_data and out_token_data both empty)
     - Uses checked arithmetic to prevent lamport calculation overflow
   - Validate and parse accounts via `Transfer2Accounts::validate_and_parse`
   - Build mint extension cache via `build_mint_extension_cache`:
     - Caches extension state for unique mints (max 5)
     - **Mode-dependent enforcement:** Compress enforces restrictions; Decompress and CompressAndClose bypass
     - For CompressAndClose with restricted extensions: requires CompressedOnly extension in output TLV

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

   c. **Close accounts for CompressAndClose operations:**
      - After compression validation succeeds, close the token accounts:
        - Lamport distribution via `AccountRentState::calculate_close_distribution()`:
          - Rent exemption + completed epoch rent → rent_sponsor account
          - Unutilized rent (partial epoch) → destination account
          - Compression incentive → forester (when rent authority closes)
        - Zero out account data and resize to 0 bytes
        - Account becomes uninitialized and can be garbage collected
        - See `program-libs/compressible/docs/RENT.md#close-account-distribution` for distribution logic

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
      - Execute `process_token_compression` (src/transfer2/compression/mod.rs) for compress/decompress operations
      - Extract CPI accounts and tree pubkeys via `validated_accounts.cpi_accounts`
      - Execute `execute_cpi_invoke` with light-system-program
      - Execute `close_for_compress_and_close` (src/transfer2/compression/ctoken/compress_and_close.rs) for CompressAndClose operations

      **CPI Context Write Path:**
      If `validated_accounts.write_to_cpi_context_system` exists:
      - Validate exactly 4 accounts provided (error: Transfer2CpiContextWriteInvalidAccess if not)
      - Accounts: [0] light-system-program, [1] fee_payer, [2] cpi_authority_pda, [3] cpi_context
      - Execute `execute_cpi_invoke` in write-only mode (no tree accounts)
      - No SOL pool operations allowed (error: Transfer2CpiContextWriteWithSolPool)

**Compression/Decompression Processing Details:**

**Key distinction between compression modes:**
- **Compress/Decompress:** Participate in sum checks - tokens are added/subtracted from running sums per mint, ensuring overall balance but no specific output validation
- **CompressAndClose:** Participates in sum checks (like Compress) AND additionally validates a specific compressed token account exists in outputs that mirrors the account being closed (same mint, amount equals full balance, owner preserved or set to account pubkey, no delegate). The output validation happens IN ADDITION to sum checks, providing extra security for account closure.

When compression processing occurs (in both Path A and Path B):

1. **Main routing logic (src/transfer2/compression/mod.rs):**
   - Function: `process_token_compression`
   - Iterate through each compression in the compressions array
   - Get source_or_recipient account from packed accounts
   - Route to handler based on account owner:
     - ctoken program → `process_ctoken_compressions` (ctoken/mod.rs)
     - SPL Token → SPL compression handler
     - SPL Token 2022 → SPL compression handler
     - Other → error (InvalidInstructionData)

2. **SPL Token compression/decompression:**
   - Validate compression mode fields (authority must be 0 for Decompress)
   - Get mint and token pool PDA from packed accounts
   - Validate pool PDA derivation matches [mint, pool_index] with provided bump
   - **For Compress:**
     - Get authority account from packed accounts
     - Transfers tokens from user's SPL token account to the token pool PDA via SPL token CPI (authority must be signer, checked by SPL program)
   - **For Decompress:**
     - Transfers tokens from the token pool PDA to recipient's SPL token account via SPL token CPI with PDA signer (CPI authority PDA signs)

3. **CToken compression/decompression (src/transfer2/compression/ctoken/):**
   - **Initial validations:**
     - Compression mode field validation (authority must be 0 for Decompress mode)
     - Account ownership verification (must be owned by ctoken program)
     - Account deserialization as CToken
     - Mint verification (account mint must match compression mint)
   - **For Compress:**
     - Validate authority via `check_authority`:
       - Check authority is signer (error: InvalidSigner)
       - If authority == owner: proceed
       - If authority == delegate: verify delegated amount ≥ compression amount, update delegation
       - Otherwise: error (OwnerMismatch)
     - Check sufficient balance (error: ArithmeticOverflow)
     - Subtracts compression amount from the source ctoken account balance (with overflow protection)
   - **For Decompress:**
     - Adds decompression amount to the recipient ctoken account balance (with overflow protection)
     - **Extension state transfer (with CompressedOnly in input TLV):**
       - Validates destination CToken is fresh (zero amount, no delegate, no close_authority)
       - Transfers delegate and delegated_amount from CompressedOnly extension to CToken
       - Transfers withheld_transfer_fee to CToken's TransferFeeAccount extension
       - Restores frozen state (sets CToken.state = 2 if extension.is_frozen)
       - Error: `DecompressDestinationNotFresh` if destination has non-zero state
     - **CompressedOnly inputs must decompress to CToken, not SPL token accounts:**
       - Error: `CompressedOnlyRequiresCTokenDecompress` if decompressing to SPL token account
   - **For CompressAndClose:**
     - **Authority validation:**
       - Authority must be signer
       - Authority must be the compression_authority (from compressible extension)
       - Owner CANNOT use CompressAndClose - only compression_authority can
       - Non-compressible accounts (without Compressible extension) CANNOT use CompressAndClose
     - **Compressibility timing check** (required gate for compression_authority):
       - Calls `compressible_ext.is_compressible(data_len, current_slot, lamports)`
       - Returns `Some(_)` if account can be compressed, `None` if not yet compressible
       - Account becomes compressible when it lacks sufficient rent for current epoch + 1
       - This prevents compression_authority from arbitrarily compressing accounts before rent expires
       - Error: `ProgramError::InvalidAccountData` with message "account not compressible" if check fails
     - **Frozen account handling:**
       - Frozen ctoken accounts (state == 2) CAN be CompressAndClose'd
       - Frozen state is preserved via CompressedOnly extension in output TLV
       - The account is temporarily unfrozen (set to initialized) to pass close validation
       - Error: `ErrorCode::AccountFrozen` if trying to Compress (not CompressAndClose) a frozen account
     - **Design principle: Compression authority control** (see `program-libs/compressible/docs/RENT.md` for detailed rent calculations)
       - Tokens: Belong to the owner, but compression is controlled by compression_authority
       - Rent exemption + completed epoch rent: Belong to rent_sponsor (who funded them)
       - Compression incentive: Goes to forester (destination) when compression_authority compresses
       - **Compressibility determination** (via `compressible::calculate_rent_and_balance`):
         - Account becomes compressible when it lacks rent for current epoch + 1
         - Compression_authority can only compress when `is_compressible()` returns true
         - See `program-libs/compressible/docs/` for complete rent system documentation
     - **Compressed token account validation - MUST exist in outputs with:**
       - Amount: Must exactly match the full token account balance being compressed
       - Owner: If `compress_to_pubkey` flag is false, owner must match original token account owner
       - Owner: If `compress_to_pubkey` flag is true, owner must be the token account's pubkey (allows closing accounts owned by PDAs)
       - **Note:** `compress_to_pubkey` is stored in the compressible extension and set during account creation, not per-instruction
       - Mint: Must match the ctoken account's mint field
       - Version: Must be ShaFlat (version=3) for security
       - Version: Must match the version specified in the token account's compressible extension
     - **Delegate/Frozen state handling (with CompressedOnly extension):**
       - If account has `compression_only` flag set (restricted mint), CompressedOnly extension is REQUIRED in output TLV
       - CompressedOnly extension preserves: `is_frozen`, `delegated_amount`, `delegate` (in token_data), `withheld_transfer_fee`
       - Delegate: Must match between ctoken.delegate and compressed output delegate
       - Delegated amount: Must match between ctoken.delegated_amount and extension.delegated_amount
       - Frozen state: Must match between ctoken.state==2 and extension.is_frozen
       - Withheld fee: Must match between ctoken TransferFeeAccount.withheld_amount and extension.withheld_transfer_fee
       - Error: `CompressAndCloseDelegatedAmountMismatch`, `CompressAndCloseInvalidDelegate`, `CompressAndCloseFrozenMismatch`, `CompressAndCloseWithheldFeeMismatch`
     - **Delegate handling (without CompressedOnly extension):**
       - Delegate: Must be None (has_delegate=false and delegate=0) - delegates cannot be carried over without extension
       - Error: `CompressAndCloseDelegateNotAllowed` if source has delegate or output has delegate
     - **Account state updates:**
       - Token account balance is set to 0
       - Account is marked for closing after the transaction
     - **Security guarantee:** CompressAndClose ensures the exact compressed account exists, preventing token loss or misdirection. Only compression_authority can initiate, protecting users from unauthorized compression.
     - **Uniqueness validation:** All CompressAndClose operations in a single instruction must use different compressed output account indices. Duplicate output indices are rejected to prevent fund theft attacks where a compression_authority could close multiple accounts but route all funds to a single compressed output
   - Calculate compressible extension top-up if present (returns Option<u64>)
   - **Transfer deduplication optimization:**
     - Collects all transfers into a 40-element array indexed by account
     - Deduplicates transfers to same account by summing amounts
     - Executes single `multi_transfer_lamports` CPI with deduplicated transfers (max 40, error: TooManyCompressionTransfers)

**Errors:**

- `ProgramError::BorshIoError` (error code: 15) - Failed to deserialize instruction data
- `ProgramError::NotEnoughAccountKeys` (error code: 11) - Missing required accounts
- `ProgramError::InvalidInstructionData` (error code: 3) - Invalid instruction data or authority index for decompress mode
- `ProgramError::InvalidAccountData` (error code: 4) - Account data deserialization fails
- `ProgramError::ArithmeticOverflow` (error code: 24) - Overflow in lamport calculations
- `CTokenError::InLamportsUnimplemented` (error code: 18050) - in_lamports field not yet implemented
- `CTokenError::OutLamportsUnimplemented` (error code: 18051) - out_lamports field not yet implemented
- `CTokenError::CompressedTokenAccountTlvUnimplemented` (error code: 18021) - out_tlv provided but not all compressions are CompressAndClose mode
- `CTokenError::CompressedOnlyBlocksTransfer` (error code: 18048) - CompressedOnly inputs cannot have compressed outputs (must decompress only)
- `CTokenError::OutTlvOutputCountMismatch` (error code: 18049) - out_tlv length does not match out_token_data length
- `CTokenError::DecompressDestinationNotFresh` (error code: 18055) - Decompress destination CToken has non-zero state (amount, delegate, etc)
- `CTokenError::InvalidInstructionData` (error code: 18001) - Compressions not allowed when writing to CPI context
- `CTokenError::InvalidCompressionMode` (error code: 18018) - Invalid compression mode value
- `CTokenError::CompressInsufficientFunds` (error code: 18019) - Insufficient balance for compression
- `CTokenError::InsufficientSupply` (error code: 18010) - Insufficient token supply for operation
- `CTokenError::ArithmeticOverflow` (error code: 18003) - Arithmetic overflow in balance calculations
- `ErrorCode::SumCheckFailed` (error code: 6005) - Input/output token amounts don't match
- `ErrorCode::InputsOutOfOrder` (error code: 6054) - Sum inputs mint indices not in ascending order
- `ErrorCode::TooManyMints` (error code: 6055) - Sum check, too many mints (max 5)
- `ErrorCode::DuplicateMint` (error code: 6056) - Duplicate mint index detected in inputs, outputs, or compressions (same mint referenced by multiple indices or same index used multiple times)
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
- `ErrorCode::CompressAndCloseInvalidOwner` (error code: 6089) - Compressed token owner does not match expected owner (source ctoken.owner or token_account.pubkey if compress_to_pubkey)
- `ErrorCode::CompressAndCloseAmountMismatch` (error code: 6090) - Compression amount must match the full token balance
- `ErrorCode::CompressAndCloseBalanceMismatch` (error code: 6091) - Token account balance must match compressed output amount
- `ErrorCode::CompressAndCloseDelegateNotAllowed` (error code: 6092) - Source token account has delegate OR compressed output has delegate (delegates not supported)
- `ErrorCode::CompressAndCloseInvalidVersion` (error code: 6093) - Compressed token version must be 3 (ShaFlat) and must match compressible extension's account_version
- `ErrorCode::CompressAndCloseInvalidMint` (error code: 6108) - Compressed token mint does not match source token account mint
- `ErrorCode::CompressAndCloseMissingCompressedOnlyExtension` (error code: 6109) - Missing required CompressedOnly extension for restricted mint or frozen account
- `ErrorCode::CompressAndCloseDelegatedAmountMismatch` (error code: 6116) - Delegated amount mismatch between ctoken and CompressedOnly extension
- `ErrorCode::CompressAndCloseInvalidDelegate` (error code: 6118) - Delegate mismatch between ctoken and compressed token output
- `ErrorCode::CompressAndCloseWithheldFeeMismatch` (error code: 6120) - Withheld transfer fee mismatch
- `ErrorCode::CompressAndCloseFrozenMismatch` (error code: 6122) - Frozen state mismatch between ctoken and CompressedOnly extension
- `ErrorCode::CompressedOnlyRequiresCTokenDecompress` (error code: 6144) - CompressedOnly inputs must decompress to CToken account, not SPL token account
- `ErrorCode::TlvRequiresVersion3` (error code: 6123) - TLV extensions only supported with version 3 (ShaFlat)
- `ErrorCode::CompressAndCloseDuplicateOutput` (error code: 6420) - Cannot use the same compressed output account for multiple CompressAndClose operations (security protection against fund theft)
- `ErrorCode::CompressAndCloseOutputMissing` (error code: 6421) - Compressed token account output required but not provided
- `AccountError::InvalidSigner` (error code: 12015) - Required signer account is not signing
- `AccountError::AccountNotMutable` (error code: 12008) - Required mutable account is not mutable
- Additional errors from close_token_account for CompressAndClose operations

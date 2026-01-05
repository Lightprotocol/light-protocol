
# Instructions

**Instruction Schema:**
1. every instruction description must include the sections:
    - **path** path to instruction code in the program
    - **description** highlevel description what the instruction does including accounts used and their state layout (paths to the code), usage flows what the instruction does
    - **instruction_data** paths to code where instruction data structs are defined
    - **Accounts** accounts in order including checks
    - **Instruction logic and checks**
    - **Errors** possible errors and description what causes these errors


## 1. create ctoken account

  **discriminator:** 18
  **enum:** `CTokenInstruction::CreateTokenAccount`
  **path:** programs/compressed-token/program/src/ctoken/create.rs

  **description:**
  1. creates ctoken solana accounts with and without Compressible extension
  2. account layout `CToken` is defined in path: program-libs/ctoken-interface/src/state/ctoken/ctoken_struct.rs
  3. extension layout `CompressionInfo` is defined in path:
  program-libs/ctoken-interface/src/state/extensions/compressible.rs
  4. A compressible token means that the ctoken solana account can be compressed by the rent authority as soon as the account balance is insufficient.
  5. Account creation without the compressible extension:
    - Initializes an existing 165-byte solana account as a ctoken account (SPL-compatible size)
    - Only sets mint, owner, and state fields - no extension data
    - Account must already exist and be owned by the program
  6. Account creation with compressible extension:
    - creates the ctoken account via cpi within the instruction, then initializes it.
    - expects a CompressibleConfig account to read the rent authority, rent recipient and RentConfig from.
    - if the payer is not the rent recipient the fee payer pays the rent and becomes the rent recipient (the rent recipient is a ctoken program pda that funds rent exemption for compressible ctoken solana accounts)

  **Instruction data:**
  1. instruction data is defined in path: program-libs/ctoken-interface/src/instructions/create_ctoken_account.rs
    - `owner`: The owner pubkey for the token account (32 bytes)
    - `compressible_config`: Optional `CompressibleExtensionInstructionData` (None = non-compressible account)
  2. Instruction data with compressible extension
  program-libs/ctoken-interface/src/instructions/extensions/compressible.rs
    - `token_account_version`: Version of the compressed token account hashing scheme (u8)
    - `rent_payment`: Number of epochs to prepay for rent (u8)
      - `rent_payment = 1` is explicitly forbidden to prevent epoch boundary timing edge case (its rent for the current rent epoch)
      - Allowed values: 0 (no prefunding) or 2+ epochs (safe buffer)
      - Rationale: Accounts created with exactly 1 epoch near epoch boundaries could become immediately compressible
    - `compression_only`: If set to non-zero, the compressed token account cannot be transferred, only decompressed (u8)
    - `write_top_up`: Additional lamports allocated for future write operations on the compressed account (u32)
    - `compress_to_account_pubkey`: Optional `CompressToPubkey` for compressing to account pubkey instead of owner

  **Accounts:**
  1. token_account
    - (signer for compressible, mutable)
    - The ctoken account being created
    - For compressible accounts: must be signer (account created via CPI)
    - For non-compressible accounts: doesn't need to be signer (SPL compatibility)
  2. mint
    - (non-mutable)
    - Mint pubkey is used for token account initialization and extension detection
    - Account is unchecked and doesn't need to be initialized, allowing compressed mints to be used without providing the compressed account

  Optional accounts required to initialize ctoken account with compressible extension:
  3. payer
    - (signer, mutable)
    - User account, pays for the compression incentive when using rent_sponsor
  4. config
    - (non-mutable)
    - Owned by LightRegistry program, CompressibleConfig::discriminator matches
    - Used to read RentConfig, rent_sponsor, and compression_authority
    - Must be in ACTIVE state
  5. system_program
    - (non-mutable)
    - Required for account creation and rent transfer
  6. rent_payer
    - (mutable)
    - Either rent_sponsor PDA or custom fee payer
    - If custom fee payer: must be signer, pays rent exemption + compression incentive
    - If rent_sponsor: not signer, pays only rent exemption (payer pays compression incentive)

  **Instruction Logic and Checks:**
  1. Deserialize instruction data
    - If instruction data len == 32 bytes, treat as owner-only (SPL Token initialize_account3 compatibility)
    - Otherwise, deserialize as `CreateTokenAccountInstructionData`
  2. Parse and check accounts based on is_compressible flag
    - For compressible: token_account must be signer
    - Validate CompressibleConfig is active (not inactive or deprecated)
  3. Check mint extensions using `has_mint_extensions()`
  4. If with compressible account:
    4.1. Validate rent_payment is not exactly 1 epoch (must cover more than the current rent epoch or be 0)
        - Check: `compressible_config.rent_payment != 1`
        - Error: `ErrorCode::OneEpochPrefundingNotAllowed` if validation fails
        - Purpose: Prevent accounts from becoming immediately compressible due to epoch boundary timing
    4.2. If with compress_to_pubkey:
        - Validates: derives address from provided seeds/bump and verifies it matches token_account pubkey
        - Security: ensures account is a derivable PDA, preventing compression to non-signable addresses
    4.3. Validate compression_only requirement for restricted extensions:
        - If mint has restricted extensions (e.g., TransferFee) and compression_only == 0
        - Error: `ErrorCode::CompressionOnlyRequired`
    4.4. Calculate account size based on mint extensions (includes Compressible extension)
    4.5. Calculate rent (rent exemption + prepaid epochs rent + compression incentive)
    4.6. Check whether rent_payer is custom fee payer (rent_payer != config.rent_sponsor)
    4.7. If custom rent payer:
        - Verify rent_payer is signer (prevents executable accounts as rent_sponsor)
        - Create account with custom rent_payer via CPI (pays both rent exemption + additional lamports)
    4.8. If using protocol rent_sponsor:
        - Create account with rent_sponsor PDA as fee payer via CPI (pays only rent exemption)
        - Transfer compression incentive to created ctoken account from payer via CPI
    4.9. `initialize_ctoken_account` (programs/compressed-token/program/src/shared/initialize_ctoken_account.rs)
        - Copy version from config (used to match config PDA version in subsequent instructions)
        - If custom fee payer, set custom fee payer as ctoken account rent_sponsor
        - Else set config.rent_sponsor as ctoken account rent_sponsor
        - Set `last_claimed_slot` to current slot (tracks when rent was last claimed/initialized)

  **Errors:**
  - `ProgramError::BorshIoError` (error code: 15) - Failed to deserialize CreateTokenAccountInstructionData from instruction_data bytes
  - `AccountError::NotEnoughAccountKeys` (error code: 12020) - Missing required accounts
  - `AccountError::InvalidSigner` (error code: 12015) - token_account or payer is not a signer when required
  - `AccountError::AccountNotMutable` (error code: 12008) - token_account or payer is not mutable when required
  - `AccountError::AccountOwnedByWrongProgram` (error code: 12007) - Config account not owned by LightRegistry program
  - `ProgramError::InvalidAccountData` (error code: 4) - CompressibleConfig pod deserialization fails or compress_to_pubkey.check_seeds() fails
  - `ProgramError::InvalidInstructionData` (error code: 3) - compressible_config is None in instruction data when compressible accounts provided, or extension data invalid
  - `ProgramError::MissingRequiredSignature` (error code: 8) - Custom rent_payer is not a signer
  - `ProgramError::UnsupportedSysvar` (error code: 17) - Failed to get Clock sysvar
  - `CompressibleError::InvalidState` (error code: 19002) - CompressibleConfig is not in active state
  - `ErrorCode::InsufficientAccountSize` (error code: 6077) - token_account data length < 165 bytes (non-compressible) or < COMPRESSIBLE_TOKEN_ACCOUNT_SIZE (compressible)
  - `ErrorCode::InvalidCompressAuthority` (error code: 6052) - compressible_config is Some but compressible_config_account is None during extension initialization
  - `ErrorCode::OneEpochPrefundingNotAllowed` (error code: 6116) - rent_payment is exactly 1 epoch, which is forbidden due to epoch boundary timing edge case
  - `ErrorCode::CompressionOnlyRequired` (error code: 6131) - Mint has restricted extensions (e.g., TransferFee) but compression_only is not set in instruction data


## 2. create associated ctoken account

  **discriminator:** 100 (non-idempotent), 102 (idempotent)
  **enum:** `CTokenInstruction::CreateAssociatedCTokenAccount` (non-idempotent), `CTokenInstruction::CreateAssociatedTokenAccountIdempotent` (idempotent)
  **path:** programs/compressed-token/program/src/ctoken/create_ata.rs

  **description:**
  1. Creates deterministic ctoken PDA accounts derived from [owner, ctoken_program_id, mint]
  2. Supports both non-idempotent (fails if exists) and idempotent (succeeds if exists) modes
  3. Account layout same as create ctoken account: `CToken` with optional `CompressionInfo`
  4. Associated token accounts cannot use compress_to_pubkey (always compress to owner)
  5. Owner and mint are provided as accounts, bump is provided via instruction data
  6. Token account must be uninitialized (owned by system program) unless idempotent mode

  **Instruction data:**
  1. instruction data is defined in path: program-libs/ctoken-interface/src/instructions/create_associated_token_account.rs
    - `bump`: PDA bump seed for derivation (u8)
    - `compressible_config`: Optional `CompressibleExtensionInstructionData`, same as create ctoken account but compress_to_account_pubkey must be None

  **Accounts:**
  1. owner
    - (non-mutable, non-signer)
    - The owner of the associated token account (used for PDA derivation and initialization)
  2. mint
    - (non-mutable, non-signer)
    - The mint for the token account (used for PDA derivation and initialization)
  3. fee_payer
    - (signer, mutable)
    - Pays for account creation and compression incentive
  4. associated_token_account
    - (mutable, NOT signer)
    - The PDA being created, must be system-owned (uninitialized) unless idempotent
  5. system_program
    - (non-mutable)
    - Required for account creation

  Optional accounts for compressible extension (same as create ctoken account):
  6. config
    - (non-mutable)
    - Owned by LightRegistry program, CompressibleConfig::discriminator matches
    - Used to read RentConfig, rent_sponsor, and compression_authority
  7. rent_payer
    - (mutable)
    - Either rent_sponsor PDA or custom fee payer (must be signer if custom)

  **Instruction Logic and Checks:**
  1. Deserialize instruction data
  2. Parse accounts: owner, mint, fee_payer, associated_token_account, system_program
  3. If idempotent mode:
    - Validate PDA derivation matches [owner, program_id, mint] with provided bump
    - Return success if account already owned by ctoken program
  4. Verify account is system-owned (uninitialized)
    - Error: `ProgramError::IllegalOwner` if not owned by system program
  5. If compressible:
    - Validate rent_payment is not exactly 1 epoch (same as create ctoken account step 3.0)
      - Check: `compressible_config.rent_payment != 1`
      - Error: `ErrorCode::OneEpochPrefundingNotAllowed` if validation fails
    - Reject if compress_to_account_pubkey is Some (not allowed for ATAs)
      - Error: `ProgramError::InvalidInstructionData` if compress_to_account_pubkey is Some
    - Parse additional accounts: config, rent_payer
    - Validate CompressibleConfig is active (not inactive or deprecated)
    - Calculate account size based on mint extensions (includes Compressible extension)
    - Calculate rent (rent exemption + prepaid epochs rent + compression incentive)
    - Check if custom rent payer (rent_payer != config.rent_sponsor)
    - If custom rent payer:
      - Verify rent_payer is signer
      - Create ATA PDA with rent_payer paying rent exemption + additional lamports
    - If using protocol rent_sponsor:
      - Create ATA PDA with rent_sponsor PDA paying rent exemption
      - Transfer compression incentive from fee_payer to account via CPI
  6. If not compressible:
    - Create ATA PDA with fee_payer paying rent exemption (base 165-byte SPL layout)
  7. Initialize token account with is_ata flag set (same as ## 1. create ctoken account step 3.6, but with is_ata=true)

  **Errors:**
  Same as create ctoken account with additions:
  - `ProgramError::IllegalOwner` (error code: 18) - Associated token account not owned by system program when creating
  - `ProgramError::InvalidInstructionData` (error code: 3) - compress_to_account_pubkey is Some (forbidden for ATAs)
  - `ProgramError::MissingRequiredSignature` (error code: 8) - Custom rent_payer is not a signer
  - `AccountError::InvalidSigner` (error code: 12015) - fee_payer is not a signer
  - `AccountError::AccountNotMutable` (error code: 12008) - fee_payer or associated_token_account is not mutable
  - `ErrorCode::OneEpochPrefundingNotAllowed` (error code: 6116) - rent_payment is exactly 1 epoch (see create ctoken account errors)


# Instructions

**Instruction Schema:**
1. every instruction description must include the sections:
    - **path** path to instruction code in the program
    - **description** highlevel description what the instruction does including accounts used and their state layout (paths to the code), usage flows what the instruction does
    - **instruction_data** paths to code where instruction data structs are defined
    - **Accounts** accounts in order including checks
    - **instruciton logic and checks**
    - **Errors** possible errors and description what causes these errors


## 1. create ctoken account

  **discriminator:** 18
  **enum:** `CTokenInstruction::CreateTokenAccount`
  **path:** programs/compressed-token/src/create_token_account.rs

  **description:**
  1. creates ctoken solana accounts with and without Compressible extension
  2. account layout `CToken` is defined in path: program-libs/ctoken-types/src/state/ctoken/ctoken_struct.rs
  3. extension layout `CompressionInfo` is defined in path:
  program-libs/ctoken-types/src/state/extensions/compressible.rs
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
  1. instruction data is defined in path: program-libs/ctoken-types/src/instructions/create_ctoken_account.rs
  2. Instruction data with compressible extension
  program-libs/ctoken-types/src/instructions/extensions/compressible.rs
    - `rent_payment`: Number of epochs to prepay for rent (u64)
      - `rent_payment = 1` is explicitly forbidden to prevent epoch boundary timing edge case
      - Allowed values: 0 (no prefunding) or 2+ epochs (safe buffer)
      - Rationale: Accounts created with exactly 1 epoch near epoch boundaries could become immediately compressible
    - `write_top_up`: Additional lamports allocated for future write operations on the compressed account

  **Accounts:**
  1. token_account
    - (signer, mutable)
    - The ctoken account being created (signer, mutable)
  2. mint
    - non mutable
    - Mint pubkey is used for token account initialization
    - Account is unchecked and doesn't need to be initialized, allowing compressed mints to be used without providing the compressed account

  Optional accounts required to initialize ctoken account with compressible extension
  3. payer
    - (signer, mutable)
    - User account, pays for the ctoken account rent and compression incentive
  4. config
    - non-mutable, owned by LightRegistry program, CompressibleConfig::discriminator matches
    - used to read RentConfig, rent recipient, and rent authority
  5. system_program
    - non mut
    - required for account creation and rent transfer
  6. rent_payer_pda
    - mutable
    - Pays rent exemption for the compressible token account creation
    - Used as PDA signer to create the ctoken account

  **Instruction Logic and Checks:**
  1. Deserialize instruction data
    - if instruction data len == 32 bytes add 1 byte padding for spl token compatibility
  2. Parse and check accounts
    - Validate CompressibleConfig is active (not inactive or deprecated)
  3. if with compressible account
    3.0. Validate rent_payment is not exactly 1 epoch
        - Check: `compressible_config.rent_payment != 1`
        - Error: `ErrorCode::OneEpochPrefundingNotAllowed` if validation fails
        - Purpose: Prevent accounts from becoming immediately compressible due to epoch boundary timing
    3.1. if with compress to pubkey
        Compress to pubkey specifies compression to account pubkey instead of the owner.
        This is useful for pda token accounts that rely on pubkey derivation but have a program wide
        authority pda as owner.
        Validates: derives address from provided seeds/bump and verifies it matches token_account pubkey
        Security: ensures account is a derivable PDA, preventing compression to non-signable addresses
    3.2. calculate rent (rent exemption + compression incentive)
    3.3. check whether fee payer is custom fee payer (rent_payer_pda != config.rent_sponsor)
    3.4. if custom fee payer
        create account with custom fee payer via cpi (pays both rent exemption + compression incentive)
    3.5. else
        3.5.1. create account with `rent_payer_pda` as fee payer via cpi (pays only rent exemption)
        3.5.2. transfer compression incentive to created ctoken account from payer via cpi
    3.6. `initialize_ctoken_account`
        programs/compressed-token/program/src/shared/initialize_ctoken_account.rs
        3.6.1. compressible extension intialization
          copy version from config (used to match config PDA version in subsequent instructions)
          if custom fee payer, set custom fee payer as ctoken account rent recipient
          else set config account rent recipient as ctoken account rent recipient
          set `last_claimed_slot` to current slot (tracks when rent was last claimed/initialized for rent calculation)

  **Errors:**
  - `ProgramError::BorshIoError` (error code: 15) - Failed to deserialize CreateTokenAccountInstructionData from instruction_data bytes
  - `AccountError::NotEnoughAccountKeys` (error code: 12020) - Missing required accounts
  - `AccountError::InvalidSigner` (error code: 12015) - token_account or payer is not a signer when required
  - `AccountError::AccountNotMutable` (error code: 12008) - token_account or payer is not mutable when required
  - `AccountError::AccountOwnedByWrongProgram` (error code: 12007) - Config account not owned by LightRegistry program
  - `ProgramError::InvalidAccountData` (error code: 4) - CompressibleConfig pod deserialization fails or compress_to_pubkey.check_seeds() fails
  - `ProgramError::InvalidInstructionData` (error code: 3) - compressible_config is None in instruction data when compressible accounts provided, or extension data invalid
  - `ProgramError::UnsupportedSysvar` (error code: 17) - Failed to get Clock sysvar
  - `CompressibleError::InvalidState` (error code: 19002) - CompressibleConfig is not in active state
  - `ErrorCode::InsufficientAccountSize` (error code: 6077) - token_account data length < 165 bytes (non-compressible) or < COMPRESSIBLE_TOKEN_ACCOUNT_SIZE (compressible)
  - `ErrorCode::InvalidCompressAuthority` (error code: 6052) - compressible_config is Some but compressible_config_account is None during extension initialization
  - `ErrorCode::OneEpochPrefundingNotAllowed` (error code: 6116) - rent_payment is exactly 1 epoch, which is forbidden due to epoch boundary timing edge case


## 2. create associated ctoken account

  **discriminator:** 100 (non-idempotent), 102 (idempotent)
  **enum:** `CTokenInstruction::CreateAssociatedTokenAccount` (non-idempotent), `CTokenInstruction::CreateAssociatedTokenAccountIdempotent` (idempotent)
  **path:** programs/compressed-token/program/src/create_associated_token_account.rs

  **description:**
  1. Creates deterministic ctoken PDA accounts derived from [owner, ctoken_program_id, mint]
  2. Supports both non-idempotent (fails if exists) and idempotent (succeeds if exists) modes
  3. Account layout same as create ctoken account: `CToken` with optional `CompressionInfo`
  4. Associated token accounts cannot use compress_to_pubkey (always compress to owner)
  5. Mint is provided via instruction data only - no account validation for compressed mint compatibility
  6. Token account must be uninitialized (owned by system program) unless idempotent mode

  **Instruction data:**
  1. instruction data is defined in path: program-libs/ctoken-types/src/instructions/create_associated_token_account.rs
    - `owner`: Owner pubkey for the associated token account
    - `mint`: Mint pubkey for the token account
    - `bump`: PDA bump seed for derivation
    - `compressible_config`: Optional, same as create ctoken account but compress_to_account_pubkey must be None

  **Accounts:**
  1. fee_payer
    - (signer, mutable)
    - Pays for account creation and compression incentive
  2. associated_token_account
    - mutable, NOT signer (it's a PDA being created)
    - Must be system-owned (uninitialized) unless idempotent
  3. system_program
    - non-mutable
    - Required for account creation

  Optional accounts for compressible extension (same as create ctoken account):
  4. config
    - non-mutable, owned by LightRegistry program
  5. fee_payer_pda
    - mutable
    - Either rent_sponsor PDA or custom fee payer

  **Instruction Logic and Checks:**
  1. Deserialize instruction data
  2. If idempotent mode:
    - Validate PDA derivation matches [owner, program_id, mint] with provided bump
    - Return success if account already owned by program
  3. Verify account is system-owned (uninitialized)
    - Validate CompressibleConfig is active (not inactive or deprecated) if compressible
  4. If compressible:
    - Validate rent_payment is not exactly 1 epoch (same as create ctoken account step 3.0)
      - Check: `compressible_config.rent_payment != 1`
      - Error: `ErrorCode::OneEpochPrefundingNotAllowed` if validation fails
    - Reject if compress_to_account_pubkey is Some (not allowed for ATAs)
    - Calculate rent (prepaid epochs rent + compression incentive, no rent exemption)
    - Check if custom fee payer (fee_payer_pda != config.rent_sponsor)
    - Create PDA with fee_payer_pda (either rent_sponsor PDA or custom fee payer) paying rent exemption
    - Always transfer calculated rent from fee_payer to account via CPI
  5. If not compressible:
    - Create PDA with rent-exempt balance only
  6. Initialize token account (same as ## 1. create ctoken account step 3.6)

  **Errors:**
  Same as create ctoken account with additions:
  - `ProgramError::IllegalOwner` (error code: 18) - Associated token account not owned by system program when creating
  - `ProgramError::InvalidInstructionData` (error code: 3) - compress_to_account_pubkey is Some (forbidden for ATAs)
  - `AccountError::InvalidSigner` (error code: 12015) - fee_payer is not a signer
  - `AccountError::AccountNotMutable` (error code: 12008) - fee_payer or associated_token_account is not mutable
  - `ErrorCode::OneEpochPrefundingNotAllowed` (error code: 6116) - rent_payment is exactly 1 epoch (see create ctoken account errors)

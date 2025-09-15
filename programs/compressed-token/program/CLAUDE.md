
# Summary
1. This is a compressed token program implementation similar to spl-token program.
2. The program supports compressed token accounts and ctoken solana accounts (decompressed compressed tokens but not spl tokens)
3. The account layout of ctoken solana accounts is the same as for spl tokens, but we implemented a custom extension Compressible.
4. Compressed mint accounts cmints support one extension TokenMetadata.

# Accounts
- Compressed tokens can be decompressed to spl tokens. Spl tokens are not explicitly listed here.
- **description**
- **discriminator**
- **state layout**
- **serialization example**
- **hashing** (only for compressed accounts)
- **derivation:** (only for pdas)
- **associated instructions** (create, close, update)

## Solana Accounts
- The compressed token program uses

### CToken
- **description**
  struct `CompressedToken`
  ctoken solana account with spl token compatible state layout
  path: `program-libs/ctoken-types/src/state/solana_ctoken.rs`
  crate: `light-ctoken-types`
- **associated instructions**
  1. `CreateTokenAccount` `18`
  2. `CloseTokenAccount` `9`
  3. `DecompressedTransfer` `3`
  4. `Transfer2` `104` - `Decompress`, `DecompressAndClose`
  5. `MintAction` `106` - `MintToDecompressed`
  6. `Claim` `107`
- **serialization example**
  borsh and zero copy deserialization deserialize the compressible extension, spl serialization only deserialize the base token data.
  zero copy: (always use in programs)
  ```rust
  use light_ctoken_types::state::solana_ctoken::CompressedToken;
  use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};

  let (token, _) = CompressedToken::zero_copy_at(&account_data)?;
  let (mut token, _) = CompressedToken::zero_copy_at_mut(&mut account_data)?;
  ```

  borsh: (always use in client non solana program code)
  ```rust
  use borsh::BorshDeserialize;
  use light_ctoken_types::state::solana_ctoken::CompressedToken;

  let token = CompressedToken::deserialize(&mut &account_data[..])?;
  ```

  spl serialization: (preferably use other serialization)
  ```rust
  use spl_pod::bytemuck::pod_from_bytes;
  use spl_token_2022::pod::PodAccount;

  let pod_account = pod_from_bytes::<PodAccount>(&account_data[..165])?;
  ```


### Associated CToken
- **description**
  struct `CompressedToken`
  ctoken solana account with spl token compatible state layout
- **derivation:**
  seeds: [owner, ctoken_program_id, mint]
- the same as `CToken`


### Compressible Config
- owned by the LightRegistry program
- defined in path `program-libs/compressible/src/config.rs`
- crate: `light-compressible`


## Compressed Accounts

### Compressed Token
- compressed token account.
- version describes the hashing and the discriminator. (program-libs/ctoken-types/src/state/token_data_version.rs)
    pub enum TokenDataVersion {
        V1 = 1u8, // discriminator [2, 0, 0, 0, 0, 0, 0, 0], // 2 le (Poseidon hashed)
        V2 = 2u8, // discriminator [0, 0, 0, 0, 0, 0, 0, 3], // 3 be (Poseidon hashed)
        ShaFlat = 3u8, // discriminator [0, 0, 0, 0, 0, 0, 0, 4], // 4 be (Sha256 hash of borsh serialized data truncated to 31 bytes so that hash is less than be bn254 field size)
    }

### Compressed Mint

## Extensions
The compressed token program supports 2 extensions.

### TokenMetadata
- Mint extension, compatible with TokenMetada extension of Token2022.
- Only available in compressed mints.

### Compressible
- Token account extension, Token2022 does not have an equivalent extension.
- Only available in ctoken solana accounts (decompressed ctokens), not in compressed token accounts.
-


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
  2. account layout `CompressedToken` is defined in path: program-libs/ctoken-types/src/state/solana_ctoken.rs
  3. extension layout `CompressibleExtension` is defined in path:
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
  3. if with compressible account
    3.1. if with compress to pubkey
        Compress to pubkey specifies compression to account pubkey instead of the owner.
        This is useful for pda token accounts that rely on pubkey derivation but have a program wide
        authority pda as owner.
        Validates: derives address from provided seeds/bump and verifies it matches token_account pubkey
        Security: ensures account is a derivable PDA, preventing compression to non-signable addresses
    3.2. calculate rent (rent exemption + compression incentive)
    3.3. check whether fee payer is custom fee payer (rent_payer_pda != config.rent_recipient)
    3.4. if custom fee payer
        create account with custom fee payer via cpi (pays both rent exemption + compression incentive)
    3.5. else
        3.5.1. create account with `rent_payer_pda` as fee payer via cpi (pays only rent exemption)
        3.5.2. transfer compression incentive to created ctoken account from payer via cpi
    3.6. `initialize_token_account`
        programs/compressed-token/program/src/shared/initialize_token_account.rs
        3.6.1. compressible extension intialization
          copy version from config (used to match config PDA version in subsequent instructions)
          if custom fee payer, set custom fee payer as ctoken account rent recipient
          else set config account rent recipient as ctoken account rent recipient
          set `last_claimed_slot` to current slot (tracks when rent was last claimed/initialized for rent calculation)

  **Errors:**
  1. `ProgramError::BorshIoError` - Failed to deserialize CreateTokenAccountInstructionData from instruction_data bytes
  2. `ProgramError::NotEnoughAccountKeys` - Missing required account (token_account, mint, or compressible accounts when compressible_config is Some)
  3. `ProgramError::MissingRequiredSignature` - token_account or payer account is not a signer when required
  4. `ProgramError::InvalidArgument` - Account is not mutable when it needs to be (token_account, payer)
  5. `ProgramError::IncorrectProgramId` - Config account owner is not LightRegistry program (pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX"))
  6. `ProgramError::InvalidAccountData` - CompressibleConfig discriminator check fails, pod deserialization fails, or compress_to_pubkey.check_seeds() fails (derived PDA doesn't match token_account pubkey)
  7. `ProgramError::InvalidInstructionData` - compressible_config is None in instruction data when compressible accounts are provided
  8. `ProgramError::InsufficientFunds` - payer.lamports < compression incentive amount when transferring via CPI
  9. `ErrorCode::InsufficientAccountSize` - token_account data length < 165 bytes (non-compressible) or < COMPRESSIBLE_TOKEN_ACCOUNT_SIZE (compressible)
  10. `ErrorCode::InvalidCompressAuthority` - compressible_config is Some but compressible_config_account is None during extension initialization
  11. `ProgramError::Custom` - System program CPI failures for CreateAccount or Transfer instructions


## 2. create associated ctoken account

  **discriminator:** 6 (non-idempotent), 101 (idempotent)
  **enum:** `CTokenInstruction::CreateAssociatedTokenAccount` (non-idempotent), `CTokenInstruction::CreateAssociatedTokenAccountIdempotent` (idempotent)
  **path:** programs/compressed-token/program/src/create_associated_token_account.rs

  **description:**
  1. Creates deterministic ctoken PDA accounts derived from [owner, ctoken_program_id, mint]
  2. Supports both non-idempotent (fails if exists) and idempotent (succeeds if exists) modes
  3. Account layout same as create ctoken account: `CompressedToken` with optional `CompressibleExtension`
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
    - Either rent_recipient PDA or custom fee payer

  **Instruction Logic and Checks:**
  1. Deserialize instruction data
  2. If idempotent mode:
    - Validate PDA derivation matches [owner, program_id, mint] with provided bump
    - Return success if account already owned by program
  3. Verify account is system-owned (uninitialized)
  4. If compressible:
    - Reject if compress_to_account_pubkey is Some (not allowed for ATAs)
    - Calculate rent (prepaid epochs rent + compression incentive, no rent exemption)
    - Check if custom fee payer (fee_payer_pda != config.rent_recipient)
    - Create PDA with fee_payer_pda (either rent_recipient PDA or custom fee payer) paying rent exemption
    - Always transfer calculated rent from fee_payer to account via CPI
  5. If not compressible:
    - Create PDA with rent-exempt balance only
  6. Initialize token account (same as ## 1. create ctoken account step 3.6)

  **Errors:**
  Same as ## 1. create ctoken account with additions:
  - `ProgramError::IllegalOwner` - Associated token account not owned by system program when creating
  - `ProgramError::InvalidInstructionData` - compress_to_account_pubkey is Some (forbidden for ATAs)

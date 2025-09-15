
# Summary
1. This is a compressed token program implementation similar to spl-token program.
2. The program supports compressed token accounts and ctoken solana accounts (decompressed compressed tokens but not spl tokens)
3. The account layout of ctoken solana accounts is the same as for spl tokens, but we implemented a custom extension Compressible.
4. Compressed mint accounts cmints support one extension TokenMetadata.

# Instructions

**Instruction Schema:**
1. every instruction description must include the sections:
    - **path** path to instruction code in the program
    - **description** highlevel description what the instruction does including accounts used and their state layout (paths to the code), usage flows what the instruction does
    - **instruction_data** paths to code where instruction data structs are defined
    - **Accounts** accounts in order including checks
    - **instruciton logic and checks**


## 1. create ctoken account

  **path:** programs/compressed-token/src/create_token_account.rs

  **description:**
  1. creates ctoken solana accounts with and without Compressible extension
  2. account layout `CompressedToken` is defined in path: program-libs/ctoken-types/src/state/solana_ctoken.rs
  3. extension layout `CompressibleExtension` is defined in path:
  program-libs/ctoken-types/src/state/extensions/compressible.rs
  4. A compressible token means that the ctoken solana account can be compressed by the rent authority as soon as the account balance is insufficient.
  5. Account creation without the compressible extension mirrors spl initialize3 instruction it just initializes an existing solana account as a ctoken account.
  6. Account creation with compressible extension:
    - creates the ctoken account via cpi within the instruction, then initializes it.
    - expects a CompressibleConfig account to read the rent authority, rent recipient and RentConfig from.
    - if the payer is not the rent recipient the fee payer pays the rent and becomes the rent recipient (the rent recipient is a ctoken program pda that funds rent exemption for compressible ctoken solana accounts)

  **Instruction data:**
  1. instruction data is defined in path: program-libs/ctoken-types/src/instructions/create_ctoken_account.rs
  2. Instruction data with compressible extension
  program-libs/ctoken-types/src/instructions/extensions/compressible.rs

  **Accounts:**
  1. token_account
    - (signer, mutable)
    - The ctoken account being created (signer, mutable)
  2. mint
    - non mutable
    - Unused account kept for spl program compatibility

  Optional accounts required to initialize ctoken account with compressible extension
  3. payer
    - (signer, mutable)
    - User account, pays for the ctoken account rent and compression incentive
  4. config
    - mutable, owned by LightRegistry program, CompressibleConfig::discriminator matches
    - used to read RentConfig, rent recipient, and rent authority
  5. system_program
    - non mut
    - required for account creation and rent transfer
  6. rent_payer_pda
    - signer, mutable
    - Pays rent exemption for the compressible token account creation.

  **Instruction Logic and Checks:**
  1. Deserialize instruction data
    - if instruction data len == 32 bytes add 1 byte padding for spl token compatibility
  2. Parse and check accounts
  3.


**IMPORTANT**: read this complete file and all referenced md files completely!


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

**Accounts:**
- you will find all accounts of and related to the CToken program in `programs/compressed-token/program/docs/ACCOUNTS.md`
1. Solana Accounts
  1.1. CToken (`CompressedToken`)
  1.2. Associated CToken (`CompressedToken`)
  1.3. Extension: Compressible Config (`CompressibleConfig`)
2. Compressed Accounts
  2.1. Compressed Token (`TokenData`)
  2.2. Compressed Mint (`CompressedMint`)




# Instructions

**Instruction Schema:**
Every instruction description must include the sections:
- **path** path to instruction code in the program
- **description** highlevel description what the instruction does including accounts used and their state layout (paths to the code), usage flows what the instruction does
- **instruction_data** paths to code where instruction data structs are defined
- **Accounts** accounts in order including checks
- **instruction logic and checks**
- **Errors** possible errors and description what causes these errors

## Instruction Index

### Account Management
1. **Create CToken Account** - [`docs/instructions/CREATE_TOKEN_ACCOUNT.md`](docs/instructions/CREATE_TOKEN_ACCOUNT.md)
   - Create regular token account (discriminator: 18, enum: `InstructionType::CreateTokenAccount`)
   - Create associated token account (discriminator: 100, enum: `InstructionType::CreateAssociatedCTokenAccount`)
   - Create associated token account idempotent (discriminator: 102, enum: `InstructionType::CreateAssociatedTokenAccountIdempotent`)
   - **Config validation:** Requires ACTIVE config only

2. **Close Token Account** - [`docs/instructions/CLOSE_TOKEN_ACCOUNT.md`](docs/instructions/CLOSE_TOKEN_ACCOUNT.md) (discriminator: 9, enum: `InstructionType::CloseTokenAccount`)
   - Close decompressed token accounts
   - Returns rent exemption to rent recipient if compressible
   - Returns remaining lamports to destination account

### Rent Management
3. **Claim** - [`docs/instructions/CLAIM.md`](docs/instructions/CLAIM.md)
   - Claims rent from expired compressible accounts (discriminator: 104, enum: `InstructionType::Claim`)
   - **Config validation:** Not inactive (active or deprecated OK)

4. **Withdraw Funding Pool** - [`docs/instructions/WITHDRAW_FUNDING_POOL.md`](docs/instructions/WITHDRAW_FUNDING_POOL.md)
   - Withdraws funds from rent recipient pool (discriminator: 105, enum: `InstructionType::WithdrawFundingPool`)
   - **Config validation:** Not inactive (active or deprecated OK)

### Token Operations
5. **Transfer2** - [`docs/instructions/TRANSFER2.md`](docs/instructions/TRANSFER2.md)
   - Batch transfer instruction for compressed/decompressed operations (discriminator: 101, enum: `InstructionType::Transfer2`)
   - Supports Compress, Decompress, CompressAndClose operations
   - Multi-mint support with sum checks

6. **MintAction** - [`docs/instructions/MINT_ACTION.md`](docs/instructions/MINT_ACTION.md)
   - Batch instruction for compressed mint management and mint operations (discriminator: 103, enum: `InstructionType::MintAction`)
   - Supports 9 action types: CreateCompressedMint, MintTo, UpdateMintAuthority, UpdateFreezeAuthority, CreateSplMint, MintToCToken, UpdateMetadataField, UpdateMetadataAuthority, RemoveMetadataKey
   - Handles both compressed and decompressed token minting

7. **CTokenTransfer** - [`docs/instructions/CTOKEN_TRANSFER.md`](docs/instructions/CTOKEN_TRANSFER.md)
   - Transfer between decompressed accounts (discriminator: 3, enum: `InstructionType::CTokenTransfer`)

8. **CTokenTransferChecked** - [`docs/instructions/CTOKEN_TRANSFER_CHECKED.md`](docs/instructions/CTOKEN_TRANSFER_CHECKED.md)
   - Transfer with decimals validation (discriminator: 12, enum: `InstructionType::CTokenTransferChecked`)

9. **CTokenApprove** - [`docs/instructions/CTOKEN_APPROVE.md`](docs/instructions/CTOKEN_APPROVE.md)
   - Approve delegate on decompressed CToken account (discriminator: 4, enum: `InstructionType::CTokenApprove`)

10. **CTokenRevoke** - [`docs/instructions/CTOKEN_REVOKE.md`](docs/instructions/CTOKEN_REVOKE.md)
   - Revoke delegate on decompressed CToken account (discriminator: 5, enum: `InstructionType::CTokenRevoke`)

11. **CTokenMintTo** - [`docs/instructions/CTOKEN_MINT_TO.md`](docs/instructions/CTOKEN_MINT_TO.md)
   - Mint tokens to decompressed CToken account (discriminator: 7, enum: `InstructionType::CTokenMintTo`)

12. **CTokenBurn** - [`docs/instructions/CTOKEN_BURN.md`](docs/instructions/CTOKEN_BURN.md)
   - Burn tokens from decompressed CToken account (discriminator: 8, enum: `InstructionType::CTokenBurn`)

13. **CTokenFreezeAccount** - [`docs/instructions/CTOKEN_FREEZE_ACCOUNT.md`](docs/instructions/CTOKEN_FREEZE_ACCOUNT.md)
   - Freeze decompressed CToken account (discriminator: 10, enum: `InstructionType::CTokenFreezeAccount`)

14. **CTokenThawAccount** - [`docs/instructions/CTOKEN_THAW_ACCOUNT.md`](docs/instructions/CTOKEN_THAW_ACCOUNT.md)
   - Thaw frozen decompressed CToken account (discriminator: 11, enum: `InstructionType::CTokenThawAccount`)

15. **CTokenApproveChecked** - [`docs/instructions/CTOKEN_APPROVE_CHECKED.md`](docs/instructions/CTOKEN_APPROVE_CHECKED.md)
   - Approve delegate with decimals validation (discriminator: 13, enum: `InstructionType::CTokenApproveChecked`)

16. **CTokenMintToChecked** - [`docs/instructions/CTOKEN_MINT_TO_CHECKED.md`](docs/instructions/CTOKEN_MINT_TO_CHECKED.md)
   - Mint tokens with decimals validation (discriminator: 14, enum: `InstructionType::CTokenMintToChecked`)

17. **CTokenBurnChecked** - [`docs/instructions/CTOKEN_BURN_CHECKED.md`](docs/instructions/CTOKEN_BURN_CHECKED.md)
   - Burn tokens with decimals validation (discriminator: 15, enum: `InstructionType::CTokenBurnChecked`)

## Config State Requirements Summary
- **Active only:** Create token account, Create associated token account
- **Not inactive:** Claim, Withdraw, Compress & Close (via registry)

# Source Code Structure (`src/`)

## Core Instructions
- **`create_token_account.rs`** - Create regular ctoken accounts with optional compressible extension
- **`create_associated_token_account.rs`** - Create deterministic ATA accounts
- **`close_token_account/`** - Close ctoken accounts, handle rent distribution
- **`transfer/`** - SPL-compatible transfers between decompressed accounts
  - `default.rs` - CTokenTransfer (discriminator: 3)
  - `checked.rs` - CTokenTransferChecked (discriminator: 12)
  - `shared.rs` - Common transfer utilities

## Token Operations
- **`transfer2/`** - Unified transfer instruction supporting multiple modes
  - `compression/` - Compress & decompress functionality
    - `ctoken/` - CToken-specific compression (compress_and_close.rs, decompress.rs, etc.)
    - `spl.rs` - SPL token compression
  - `processor.rs` - Main instruction processor
  - `accounts.rs` - Account validation and parsing
- **`mint_action/`** - Mint tokens to compressed/decompressed accounts
- **`ctoken_approve_revoke.rs`** - CTokenApprove (4), CTokenRevoke (5), CTokenApproveChecked (13)
- **`ctoken_mint_to.rs`** - CTokenMintTo (7), CTokenMintToChecked (14)
- **`ctoken_burn.rs`** - CTokenBurn (8), CTokenBurnChecked (15)
- **`ctoken_freeze_thaw.rs`** - CTokenFreezeAccount (10), CTokenThawAccount (11)

## Rent Management
- **`claim.rs`** - Claim rent from expired compressible accounts
- **`withdraw_funding_pool.rs`** - Withdraw funds from rent recipient pool

## Shared Components
- **`shared/`** - Common utilities used across instructions
  - `initialize_ctoken_account.rs` - Token account initialization with extensions
  - `create_pda_account.rs` - PDA creation and validation
  - `transfer_lamports.rs` - Safe lamport transfer helpers
  - `compressible_top_up.rs` - Rent top-up calculations for compressible accounts
  - `owner_validation.rs` - Owner and delegate authority checks
  - `token_input.rs` / `token_output.rs` - Token data handling utilities
- **`extensions/`** - Extension handling (compressible, metadata, mint extensions)
  - `mod.rs` - Extension validation and processing
  - `check_mint_extensions.rs` - T22 mint extension validation
  - `token_metadata.rs` - Token metadata extension handling
  - `processor.rs` - Extension processing utilities
- **`lib.rs`** - Main entry point and instruction dispatch (contains `InstructionType` enum)

## Data Structures
All state and instruction data structures are defined in **`program-libs/ctoken-interface/`** (`light-ctoken-interface` crate):
- **`state/`** - Account state structures
  - `compressed_token/` - TokenData, hashing
  - `ctoken/` - CToken (decompressed account) structure
  - `mint/` - CompressedMint structure
  - `extensions/` - Extension data (Compressible, TokenMetadata, CompressedOnly, etc.)
- **`instructions/`** - Instruction data structures for all operations
  - `transfer2/` - Transfer2 instruction data
  - `mint_action/` - MintAction instruction data
  - `extensions/` - Extension instruction data

**Why separate crate:** Data structures are isolated from program logic so SDKs can import types without pulling in program dependencies.

## Error Codes
Custom error codes are defined in **`programs/compressed-token/anchor/src/lib.rs`** (`anchor_compressed_token::ErrorCode` enum):
- Contains all program-specific error codes used across compressed token operations
- Errors are returned as `ProgramError::Custom(error_code as u32)` on-chain
- CToken-specific errors are also defined in **`program-libs/ctoken-interface/src/error.rs`** (`CTokenError` enum)

## SDKs (`sdk-libs/`)
- **`ctoken-sdk/`** - SDK for programs to interact with compressed tokens (CPIs, instruction builders)
- **`token-client/`** - Client SDK for Rust applications (test helpers, transaction builders)
- **`ctoken-types/`** - Lightweight types for client-side usage

## Compressible Extension Documentation
When working with ctoken accounts that have the compressible extension (rent management), you **MUST** read:
- **`program-libs/compressible/docs/`** - Complete rent system documentation
  - `RENT.md` - Rent calculations, compressibility checks, lamport distribution
  - `CONFIG_ACCOUNT.md` - CompressibleConfig account structure
  - `SOLANA_RENT.md` - Comparison of Solana vs Light Protocol rent systems
- **Key concepts:**
  - Rent authority can compress accounts only when `is_compressible()` returns true
  - Lamport distribution on close: rent → rent_sponsor, unutilized → destination
  - Compression incentive for foresters when rent authority compresses

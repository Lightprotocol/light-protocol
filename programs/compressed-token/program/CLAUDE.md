
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
   - Create regular token account (discriminator: 18, enum: `CTokenInstruction::CreateTokenAccount`)
   - Create associated token account (discriminator: 6, enum: `CTokenInstruction::CreateAssociatedTokenAccount`)
   - Create associated token account idempotent (discriminator: 101, enum: `CTokenInstruction::CreateAssociatedTokenAccountIdempotent`)
   - **Config validation:** Requires ACTIVE config only

2. **Close Token Account** - `src/close_token_account.rs` (discriminator: 9, enum: `CTokenInstruction::CloseTokenAccount`)
   - Close decompressed token accounts
   - Returns rent exemption to rent recipient if compressible
   - Returns remaining lamports to destination account

### Rent Management
3. **Claim** - [`docs/instructions/CLAIM.md`](docs/instructions/CLAIM.md)
   - Claims rent from expired compressible accounts (discriminator: 104, enum: `CTokenInstruction::Claim`)
   - **Config validation:** Not inactive (active or deprecated OK)

4. **Withdraw Funding Pool** - [`docs/instructions/WITHDRAW_FUNDING_POOL.md`](docs/instructions/WITHDRAW_FUNDING_POOL.md)
   - Withdraws funds from rent recipient pool (discriminator: 105, enum: `CTokenInstruction::WithdrawFundingPool`)
   - **Config validation:** Not inactive (active or deprecated OK)

### Token Operations
5. **Transfer2** - [`docs/instructions/TRANSFER2.md`](docs/instructions/TRANSFER2.md)
   - Batch transfer instruction for compressed/decompressed operations (discriminator: 101, enum: `CTokenInstruction::Transfer2`)
   - Supports Compress, Decompress, CompressAndClose operations
   - Multi-mint support with sum checks

6. **MintAction** - [`docs/instructions/MINT_ACTION.md`](docs/instructions/MINT_ACTION.md)
   - Batch instruction for compressed mint management and mint operations (discriminator: 103, enum: `CTokenInstruction::MintAction`)
   - Supports 9 action types: CreateCompressedMint, MintTo, UpdateMintAuthority, UpdateFreezeAuthority, CreateSplMint, MintToCToken, UpdateMetadataField, UpdateMetadataAuthority, RemoveMetadataKey
   - Handles both compressed and decompressed token minting

7. **CTokenTransfer** - `src/ctoken_transfer.rs` (discriminator: 3, enum: `CTokenInstruction::CTokenTransfer`)
   - Transfer between decompressed accounts

## Config State Requirements Summary
- **Active only:** Create token account, Create associated token account
- **Not inactive:** Claim, Withdraw, Compress & Close (via registry)

# Source Code Structure (`src/`)

## Core Instructions
- **`create_token_account.rs`** - Create regular ctoken accounts with optional compressible extension
- **`create_associated_token_account.rs`** - Create deterministic ATA accounts
- **`close_token_account/`** - Close ctoken accounts, handle rent distribution
- **`ctoken_transfer.rs`** - SPL-compatible transfers between decompressed accounts

## Token Operations
- **`transfer2/`** - Unified transfer instruction supporting multiple modes
  - `native_compression/` - Compress & close functionality
  - `delegate/` - Delegated transfer authorization
- **`mint_action/`** - Mint tokens to compressed/decompressed accounts

## Rent Management
- **`claim/`** - Claim rent from expired compressible accounts
- **`withdraw_funding_pool.rs`** - Withdraw funds from rent recipient pool

## Shared Components
- **`shared/`** - Common utilities used across instructions
  - `initialize_ctoken_account.rs` - Token account initialization with extensions
  - `create_pda_account.rs` - PDA creation and validation
  - `transfer_lamports.rs` - Safe lamport transfer helpers
- **`extensions/`** - Extension handling (compressible, metadata)
- **`constants.rs`** - Program seeds and constants
- **`lib.rs`** - Main entry point and instruction dispatch

## Data Structures
All state and instruction data structures are defined in **`program-libs/ctoken-types/`** (`light-ctoken-types` crate):
- **`state/`** - Account state structures (CompressedToken, TokenData, CompressedMint)
- **`instructions/`** - Instruction data structures for all operations
- **`state/extensions/`** - Extension data (Compressible, TokenMetadata)

**Why separate crate:** Data structures are isolated from program logic so SDKs can import types without pulling in program dependencies.

## Error Codes
Custom error codes are defined in **`programs/compressed-token/anchor/src/lib.rs`** (`anchor_compressed_token::ErrorCode` enum):
- Contains all program-specific error codes used across compressed token operations
- Errors are returned as `ProgramError::Custom(error_code as u32)` on-chain

## SDKs (`sdk-libs/`)
- **`compressed-token-sdk/`** - SDK for programs to interact with compressed tokens (CPIs, instruction builders)
- **`token-client/`** - Client SDK for Rust applications (test helpers, transaction builders)

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


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
3. **Claim** - `src/claim/` (discriminator: 107, enum: `CTokenInstruction::Claim`)
   - Claims rent from expired compressible accounts
   - **Config validation:** Not inactive (active or deprecated OK)

4. **Withdraw Funding Pool** - `src/withdraw_funding_pool.rs` (discriminator: 108, enum: `CTokenInstruction::WithdrawFundingPool`)
   - Withdraws funds from rent recipient pool
   - **Config validation:** Not inactive (active or deprecated OK)

### Token Operations
5. **Transfer2** - `src/transfer2/` (discriminator: 104, enum: `CTokenInstruction::Transfer2`)
   - Compress, Decompress, DecompressAndClose operations

6. **MintAction** - `src/mint_action/` (discriminator: 106, enum: `CTokenInstruction::MintAction`)
   - Mint to compressed/decompressed accounts

7. **DecompressedTransfer** - `src/decompressed_transfer.rs` (discriminator: 3, enum: `CTokenInstruction::DecompressedTransfer`)
   - Transfer between decompressed accounts

## Config State Requirements Summary
- **Active only:** Create token account, Create associated token account
- **Not inactive:** Claim, Withdraw, Compress & Close (via registry)

# Source Code Structure (`src/`)

## Core Instructions
- **`create_token_account.rs`** - Create regular ctoken accounts with optional compressible extension
- **`create_associated_token_account.rs`** - Create deterministic ATA accounts
- **`close_token_account/`** - Close ctoken accounts, handle rent distribution
- **`decompressed_token_transfer.rs`** - SPL-compatible transfers between decompressed accounts

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
  - `initialize_token_account.rs` - Token account initialization with extensions
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

## SDKs (`sdk-libs/`)
- **`compressed-token-sdk/`** - SDK for programs to interact with compressed tokens (CPIs, instruction builders)
- **`token-client/`** - Client SDK for Rust applications (test helpers, transaction builders)

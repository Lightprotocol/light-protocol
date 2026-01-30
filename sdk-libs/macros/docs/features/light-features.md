# Light Protocol Macro Features

This document covers the macro features available in Light Protocol's macro system for creating compressed (rent-free) accounts and tokens.

## Overview

Light Protocol's macros enable developers to create compressed accounts that store data off-chain in Merkle trees while maintaining full Solana composability. These macros work alongside Anchor's account macros.

```rust
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::{LightAccount, LightDiscriminator, LightHasherSha};

#[derive(Default, Debug, InitSpace, LightAccount, LightDiscriminator, LightHasherSha)]
#[account]
pub struct MyAccount {
    pub compression_info: CompressionInfo,  // Non-Option, first or last field
    pub data: u64,
}
```

---

## Accounts Struct Macros

### 1. `#[derive(LightAccounts)]`

**Purpose**: Generates `LightPreInit` and `LightFinalize` trait implementations for Anchor Accounts structs.

**Generates**:
- `light_pre_init()` - Runs after `try_accounts()`, before instruction handler
- `light_finalize()` - Runs after instruction handler completes
- Registration logic for compressed account addresses
- CPI calls for compressed mints and token accounts

**Example**:
```rust
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateParams)]
pub struct Create<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    #[account(init, payer = fee_payer, space = 8 + UserRecord::INIT_SPACE, seeds = [b"user", params.owner.as_ref()], bump)]
    #[light_account(init)]
    pub user_record: Account<'info, UserRecord>,
}
```

---

### 2. `#[light_account(init)]`

**Purpose**: Field attribute that marks an account for compression as a PDA.

**Behavior**:
- Registers the compressed account address in `light_pre_init()`
- Finalizes compression in `light_finalize()`
- Works with Anchor's `#[account(init, seeds = [...], bump)]`

**Example**:
```rust
#[account(init, payer = fee_payer, space = 8 + MyData::INIT_SPACE, seeds = [b"my_data", authority.key().as_ref()], bump)]
#[light_account(init)]
pub my_account: Account<'info, MyData>,
```

---

### 3. `#[light_account(token::...)]`

**Purpose**: Field attribute for PDA-owned token accounts (vaults).

**Behavior**:
- Generates token account handling in `light_pre_init()` and `light_finalize()`
- Supports custom authority seeds
- Works with rent-free token accounts

**Example**:
```rust
#[derive(Accounts, LightAccounts)]
pub struct CreateVault<'info> {
    #[account(
        mut,
        seeds = [b"vault", mint.key().as_ref()],
        bump
    )]
    #[light_account(token::authority = [b"vault_authority"])]
    pub vault: UncheckedAccount<'info>,
}
```

---

### 4. `#[light_account(init, mint::...)]`

**Purpose**: Field attribute that creates a compressed mint.

**Behavior**:
- Generates mint initialization in `light_pre_init()`
- Creates compressed mint via CPI to compressed token program
- Links on-chain mint PDA to compressed representation

**Key insight**: Unlike Anchor's `mint::*` which runs during `try_accounts()`, `#[light_account(init, mint::...)]` runs in `light_pre_init()` AFTER account deserialization.

**Example**:
```rust
#[derive(Accounts, LightAccounts)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Initialized in light_pre_init
    #[account(mut)]
    #[light_account(init, mint::decimals = 6, mint::authority = payer)]
    pub mint: UncheckedAccount<'info>,

    pub compressed_token_program: Program<'info, CompressedToken>,
    pub system_program: Program<'info, System>,
}
```

**Why UncheckedAccount**: The mint doesn't exist during `try_accounts()`. It's created later in `light_pre_init()`, so typed `Account<'info, Mint>` would fail deserialization.

---

### 5. `#[light_program]`

**Purpose**: Program-level attribute that generates compression lifecycle hooks.

**Generates**:
- `light_pre_init()` function for pre-instruction setup
- `light_finalize()` function for post-instruction cleanup
- Account registration with Light system program
- CPI builders for compression operations

**Example**:
```rust
#[light_program]
#[program]
pub mod my_program {
    use super::*;

    pub fn create_profile(ctx: Context<CreateProfile>) -> Result<()> {
        // Business logic here
        // Compression handled automatically in lifecycle hooks
        Ok(())
    }
}
```

---

## Lifecycle Hooks

### 6. `LightPreInit` Trait / `light_pre_init()`

**Purpose**: Hook that runs AFTER `try_accounts()` but BEFORE instruction handler.

**Responsibilities**:
- Register compressed account addresses
- Create compressed mints
- Initialize compression infrastructure

**Generated for instructions with compressible accounts**:
```rust
// Generated pseudo-code
impl<'info> LightPreInit<'info> for CreateProfile<'info> {
    fn light_pre_init(&mut self, /* params */) -> Result<()> {
        // 1. Derive compressed account address
        // 2. Register with Light system program
        // 3. Create compressed mint if #[light_account(init)]
        Ok(())
    }
}
```

---

### 7. `LightFinalize` Trait / `light_finalize()`

**Purpose**: Hook that runs AFTER instruction handler completes.

**Responsibilities**:
- Serialize account state for compression
- Create/update compressed account entries
- Handle compression proofs

**Example flow**:
```
try_accounts() -> light_pre_init() -> handler() -> light_finalize()
```

---

## Data Struct Derive Macros

### 8. `#[derive(LightAccount)]`

**Purpose**: Unified trait implementation for compressible account data structs.

**Generates**:
- `LightAccount` trait implementation with pack/unpack, compression_info accessors
- `PackedT` struct (Pubkeys -> u8 indices, compression_info excluded to save 24 bytes)
- `impl LightAccount for T` with space check (INIT_SPACE <= 800 bytes)

**Example**:
```rust
#[derive(Default, Debug, InitSpace, LightAccount, LightDiscriminator, LightHasherSha)]
#[account]
pub struct UserRecord {
    pub compression_info: CompressionInfo,  // Non-Option, first or last field
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub score: u64,
}
```

**Requirements**:
- `compression_info` field must be non-Option `CompressionInfo` type
- `compression_info` must be first or last field in the struct
- Combine with `LightDiscriminator` and `LightHasherSha` derives

---

### 9. `#[derive(LightDiscriminator)]`

**Purpose**: Generates unique 8-byte discriminator for account type identification.

**Behavior**:
- Creates `DISCRIMINATOR` constant
- Used for account type verification

**Example**:
```rust
#[derive(LightDiscriminator)]
pub struct UserRecord {
    pub owner: Pubkey,
    pub score: u64,
}
```

---

### 10. `#[derive(LightHasherSha)]`

**Purpose**: SHA256 variant of hashing for Light accounts.

**Generates**:
- `DataHasher` trait implementation using SHA256
- `ToByteArray` trait implementation
- `hash()` method for Merkle leaf creation

**Example**:
```rust
#[derive(LightHasherSha)]
pub struct GameState {
    pub player: Pubkey,  // No #[hash] needed - SHA256 serializes full struct
    pub level: u32,
}
```

---

### 11. `#[derive(Compressible)]`

**Purpose**: Implements all required traits for compressible accounts.

**Generates**:
- `HasCompressionInfo` trait implementation
- `Size` trait implementation
- `CompressAs` trait implementation (if `#[compress_as(...)]` attribute present)

**Example**:
```rust
#[derive(Compressible)]
#[compress_as(start_time = 0, end_time = None, score = 0)]
pub struct GameSession {
    pub compression_info: Option<CompressionInfo>,
    pub session_id: u64,        // KEPT
    pub player: Pubkey,         // KEPT
    pub start_time: u64,        // RESET to 0
    pub end_time: Option<u64>,  // RESET to None
    pub score: u64,             // RESET to 0
}
```

---

### 12. `#[derive(HasCompressionInfo)]`

**Purpose**: Implements accessors for the `compression_info` field.

**Generates**:
- `compression_info()` getter
- `compression_info_mut()` mutable getter

**Requirements**:
- Struct must have exactly one field named `compression_info` of type `Option<CompressionInfo>`

**Example**:
```rust
#[derive(HasCompressionInfo)]
pub struct MyAccount {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    pub data: u64,
}
```

---

## Program-Level Features

### 13. `#[light_program]`

**Purpose**: Program-level attribute that auto-discovers Light accounts and wraps instruction handlers.

**Generates**:
- `LightAccountVariant` enum for all discovered light accounts
- Seeds structs for PDA derivation
- `compress` instruction for compressing on-chain accounts
- `decompress` instruction for decompressing accounts
- Config instructions for managing compression trees
- Automatic instruction handler wrapping with `light_pre_init`/`light_finalize`

**Example**:
```rust
#[light_program]
#[program]
pub mod my_program {
    pub mod instruction_accounts;  // Macro reads this file!
    pub mod state;

    use instruction_accounts::*;
    use state::*;

    pub fn create_user(ctx: Context<CreateUser>, params: Params) -> Result<()> {
        // Your business logic
        // Compression handled automatically
    }
}
```

**Behavior**:
1. Scans the crate's `src/` directory for `#[derive(LightAccounts)]` structs
2. Extracts seeds from `#[account(seeds = [...])]` on `#[light_account(init)]` fields
3. Auto-wraps instruction handlers that use those Accounts structs
4. Generates all necessary types, enums, and instruction handlers

---

### 14. Infrastructure Field Detection

**Purpose**: Automatically detects required Light Protocol program accounts by naming convention.

**Detected fields**:
- `fee_payer` or `payer` - Fee payer account
- `compression_config` - Compression configuration
- `light_system_program` - Core compression logic
- `account_compression_program` - Merkle tree management
- `compressed_token_program` - Token compression
- `registered_program_pda` - Program registration
- `system_program` - Solana system program

**Behavior**: If these fields are present in the Accounts struct, the macros wire them into CPIs automatically.

---

### 15. Seed Classification

**Purpose**: Classifies seed expressions from `#[account(seeds = [...])]` into categories.

**Seed types**:
- `Literal` - `b"literal"` or `"string"` (hardcoded bytes)
- `Constant` - `CONSTANT` or `path::CONSTANT` (uppercase identifier)
- `CtxRooted` - `authority.key().as_ref()` (from Accounts struct field)
- `DataRooted` - `params.owner.as_ref()` (from instruction parameter)
- `FunctionCall` - `max_key(&params.key_a, &params.key_b).as_ref()` (dynamic function)
- `Passthrough` - Everything else (complex expressions)

**Usage**: Used by `#[light_program]` to generate seeds structs and compress/decompress instructions.

---

### 16. Variant Enum Generation

**Purpose**: Creates `LightAccountVariant` enum for all discovered light accounts.

**Generated by**: `#[light_program]` macro

**Example**:
```rust
// Generated enum
pub enum LightAccountVariant {
    UserRecord(crate::state::UserRecord),
    GameSession(crate::state::GameSession),
    // ... one variant per light account type
}

impl LightAccountVariant {
    pub fn discriminator(&self) -> [u8; 8] { /* ... */ }
    pub fn hash(&self) -> [u8; 32] { /* ... */ }
    pub fn pack(&self, accounts: &[Pubkey]) -> Vec<u8> { /* ... */ }
    // ...
}
```

---

### 17. Compress/Decompress Instructions

**Purpose**: Auto-generated instructions for compressing/decompressing accounts.

**Generated by**: `#[light_program]` macro

**Compress instruction**:
```rust
pub fn compress(
    ctx: Context<CompressAccounts>,
    variant: LightAccountVariant,
    seeds: SeedsEnum,
) -> Result<()> {
    // Generated compression logic
}
```

**Decompress instruction**:
```rust
pub fn decompress(
    ctx: Context<DecompressAccounts>,
    variant: LightAccountVariant,
    seeds: SeedsEnum,
) -> Result<()> {
    // Generated decompression logic
}
```

---

## Execution Flow Comparison

### Anchor Standard Flow
```
try_accounts() {
    1. Extract AccountInfo
    2. Create via system CPI (init)
    3. Init token/mint CPI (mint::*, token::*)
    4. Deserialize
}
// instruction handler
exit() {
    5. Close accounts (if close = ...)
}
```

### Light Protocol Flow
```
try_accounts() {
    1. Extract AccountInfo
    2. Create PDA via system CPI (if init)
    3. Deserialize
}
light_pre_init() {
    4. Register compressed address
    5. Create compressed mint CPI (if #[light_account(init, mint::...)])
}
// instruction handler
light_finalize() {
    6. Complete compression
    7. Update Merkle trees
}
exit() {
    8. Close accounts (if close = ...)
}
```

---

## Complete Example

```rust
use anchor_lang::prelude::*;
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::*;

// Data struct with compression support
#[derive(Default, Debug, InitSpace, LightAccount, LightDiscriminator, LightHasherSha)]
#[account]
pub struct UserProfile {
    pub compression_info: CompressionInfo,  // Non-Option, first or last field
    pub owner: Pubkey,
    #[max_len(32)]
    pub username: String,
    pub level: u8,
}

#[light_program]
#[program]
pub mod my_program {
    use super::*;

    pub fn create_profile(ctx: Context<CreateProfile>, username: String) -> Result<()> {
        let profile = &mut ctx.accounts.profile;
        profile.owner = ctx.accounts.user.key();
        profile.username = username;
        profile.level = 1;
        Ok(())
    }
}

// Accounts struct with Light support
#[derive(Accounts, LightAccounts)]
pub struct CreateProfile<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        space = 8 + UserProfile::INIT_SPACE,
        seeds = [b"profile", user.key().as_ref()],
        bump
    )]
    #[light_account(init)]
    pub profile: Account<'info, UserProfile>,

    pub system_program: Program<'info, System>,

    // Light Protocol infrastructure (auto-detected)
    pub light_system_program: Program<'info, LightSystem>,
    pub account_compression_program: Program<'info, AccountCompression>,
}
```

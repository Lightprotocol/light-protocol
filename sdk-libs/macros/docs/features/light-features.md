# Light Protocol RentFree Features

This document covers the 17 features available in Light Protocol's rentfree macro system for creating compressed (rent-free) accounts and tokens.

## Overview

Light Protocol's rentfree macros enable developers to create compressed accounts that store data off-chain in Merkle trees while maintaining full Solana composability. These macros work alongside or as replacements for Anchor's account macros.

```rust
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::{RentFree, Compressible, HasCompressionInfo};

#[derive(RentFree, Compressible, HasCompressionInfo)]
#[light_account(init)]
pub struct MyAccount {
    pub data: u64,
    #[compression_info]
    pub compression_info: CompressionInfo,
}
```

---

## Account-Level Macros

### 1. `#[derive(LightAccounts)]`

**Purpose**: Generates the core traits needed for a compressible account.

**Generates**:
- Serialization/deserialization implementations
- Account discriminator handling
- Pack/unpack logic for Solana accounts

**Example**:
```rust
#[derive(LightAccounts)]
pub struct UserProfile {
    pub name: [u8; 32],
    pub score: u64,
}
```

---

### 2. `#[light_account(init)]`

**Purpose**: Attribute for account structs that marks fields and configures compression behavior.

**Supported field attributes**:
- `#[compression_info]` - Marks the CompressionInfo field
- `#[compress_as(...)]` - Specifies how to hash a field

**Example**:
```rust
#[derive(LightAccounts)]
#[light_account(init)]
pub struct GameState {
    #[compress_as(pubkey)]
    pub player: Pubkey,
    pub level: u8,
    #[compression_info]
    pub compression_info: CompressionInfo,
}
```

---

### 3. `#[light_account(token)]`

**Purpose**: Marks an account as a token account that can be compressed/decompressed.

**Behavior**:
- Generates token-specific pack/unpack implementations
- Integrates with compressed token program
- Handles token account state serialization

**Example**:
```rust
#[derive(Accounts, LightAccounts)]
pub struct CreateVault<'info> {
    #[account(
        mut,
        seeds = [b"vault", mint.key().as_ref()],
        bump
    )]
    #[light_account(token, authority = [b"vault_authority"])]
    pub vault: UncheckedAccount<'info>,
}
```

---

### 4. `#[light_mint]`

**Purpose**: Creates a compressed mint alongside an on-chain mint PDA.

**Behavior**:
- Generates mint initialization in `light_pre_init()`
- Creates compressed mint via CPI to compressed token program
- Links on-chain mint to compressed representation

**Key insight**: Unlike Anchor's `mint::*` which runs during `try_accounts()`, `#[light_mint]` runs in `light_pre_init()` AFTER account deserialization.

**Example**:
```rust
#[derive(Accounts)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Initialized in light_pre_init
    #[account(mut)]
    #[light_account(init, mint,
        decimals = 6,
        authority = payer,
        freeze_authority = payer
    )]
    pub mint: UncheckedAccount<'info>,

    pub compressed_token_program: Program<'info, CompressedToken>,
    pub system_program: Program<'info, System>,
}
```

**Why UncheckedAccount**: The mint doesn't exist during `try_accounts()`. It's created later in `light_pre_init()`, so typed `Account<'info, Mint>` would fail deserialization.

---

### 5. `#[rentfree_program]`

**Purpose**: Program-level attribute that generates compression lifecycle hooks.

**Generates**:
- `light_pre_init()` function for pre-instruction setup
- `light_finalize()` function for post-instruction cleanup
- Account registration with Light system program
- CPI builders for compression operations

**Example**:
```rust
#[rentfree_program]
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
        // 3. Create compressed mint if #[light_mint]
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

### 8. `#[derive(Compressible)]`

**Purpose**: Implements the `Compressible` trait for hashing account data into Merkle leaves.

**Generates**:
- `to_compressed_data()` method
- Field-by-field hashing logic
- Poseidon hash tree construction

**Example**:
```rust
#[derive(Compressible)]
pub struct ProfileData {
    pub name: [u8; 32],
    pub level: u8,
}
```

---

### 9. `#[derive(CompressiblePack)]`

**Purpose**: Combines `Compressible` with serialization for storage.

**Generates**:
- `Compressible` implementation
- Borsh serialization
- Pack/unpack for Solana account data

**Example**:
```rust
#[derive(CompressiblePack)]
pub struct GameSession {
    pub player: Pubkey,
    pub score: u64,
    pub completed: bool,
}
```

---

### 10. `#[derive(LightCompressible)]`

**Purpose**: Full compression support including address derivation.

**Generates**:
- All `Compressible` functionality
- Address derivation helpers
- Merkle tree integration

**Example**:
```rust
#[derive(LightCompressible)]
pub struct CompressedUserData {
    pub owner: Pubkey,
    pub data: [u8; 64],
}
```

---

### 11. `#[derive(HasCompressionInfo)]`

**Purpose**: Implements accessors for the `CompressionInfo` field.

**Generates**:
- `compression_info()` getter
- `compression_info_mut()` mutable getter
- Field detection from `#[compression_info]` attribute

**Example**:
```rust
#[derive(HasCompressionInfo)]
pub struct MyAccount {
    pub data: u64,
    #[compression_info]
    pub compression_info: CompressionInfo,
}
```

---

### 12. `#[derive(CompressAs)]`

**Purpose**: Derives hashing behavior based on `#[compress_as(...)]` field attributes.

**Supported compress_as types**:
- `pubkey` - Hash as 32-byte pubkey
- `u64` / `u128` - Hash as integer
- `bytes` - Hash as raw bytes
- `array` - Hash array elements

**Example**:
```rust
#[derive(CompressAs)]
pub struct MixedData {
    #[compress_as(pubkey)]
    pub owner: Pubkey,
    #[compress_as(u64)]
    pub amount: u64,
    #[compress_as(bytes)]
    pub metadata: [u8; 32],
}
```

---

## Infrastructure Detection

### 13. Automatic Program Detection

**Purpose**: Macros automatically detect required Light Protocol programs.

**Detected programs**:
- `light_system_program` - Core compression logic
- `account_compression_program` - Merkle tree management
- `compressed_token_program` - Token compression (if using tokens)
- `registered_program_pda` - Program registration

**Behavior**: If these accounts are present in the Accounts struct, the macros wire them into CPIs automatically.

---

### 14. Account Validation Generation

**Purpose**: Generates validation checks similar to Anchor constraints.

**Generated validations**:
- Ownership checks for compressed accounts
- Address derivation verification
- Compression state validation

**Example generated code**:
```rust
// Pseudo-code for generated validation
if account.compression_info.is_compressed {
    verify_merkle_proof(&account, &proof)?;
}
```

---

### 15. CPI Context Generation

**Purpose**: Automatically builds CPI contexts for Light Protocol operations.

**Generated for**:
- `compress_account` CPI
- `decompress_account` CPI
- `create_compressed_mint` CPI
- `transfer_compressed` CPI

**Example**:
```rust
// Generated CPI builder
fn build_compress_cpi<'info>(
    accounts: &MyAccounts<'info>,
    data: CompressedAccountData,
) -> CpiContext<'_, '_, '_, 'info, CompressAccount<'info>> {
    // Auto-generated from account struct
}
```

---

### 16. Seed Parameter Structs

**Purpose**: Generates structs for PDA seed management.

**Behavior**:
- Extracts seed fields from account definitions
- Creates typed seed parameter structs
- Integrates with address derivation

**Example**:
```rust
// Generated from:
// #[account(seeds = [b"profile", user.key().as_ref()])]

pub struct ProfileSeeds {
    pub user: Pubkey,
}

impl ProfileSeeds {
    pub fn to_seeds(&self) -> [&[u8]; 2] {
        [b"profile", self.user.as_ref()]
    }
}
```

---

### 17. Variant Enum Generation

**Purpose**: Creates enum variants for instruction dispatch with compression support.

**Behavior**:
- Generates instruction enum with compression variants
- Handles both compressed and on-chain paths
- Integrates with Anchor's instruction dispatch

**Example**:
```rust
// Generated enum
pub enum MyProgramInstruction {
    CreateProfile,
    CreateProfileCompressed,  // Compressed variant
    UpdateProfile,
    CompressProfile,          // Compression instruction
    DecompressProfile,        // Decompression instruction
}
```

---

## Execution Flow Comparison

### Anchor Standard Flow
```
try_accounts() {
    1. Extract AccountInfo
    2. Create via system CPI (init)
    3. Init token/mint CPI
    4. Deserialize
}
// instruction handler
```

### Light RentFree Flow
```
try_accounts() {
    1. Extract AccountInfo
    2. Create PDA via system CPI (if init)
    3. Deserialize
}
light_pre_init() {
    4. Register compressed address
    5. Create compressed mint CPI (if #[light_mint])
}
// instruction handler
light_finalize() {
    6. Complete compression
}
```

---

## Complete Example

```rust
use anchor_lang::prelude::*;
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::*;

#[derive(RentFree, Compressible, HasCompressionInfo)]
#[light_account(init)]
pub struct UserProfile {
    #[compress_as(pubkey)]
    pub owner: Pubkey,
    pub username: [u8; 32],
    pub level: u8,
    #[compression_info]
    pub compression_info: CompressionInfo,
}

#[rentfree_program]
#[program]
pub mod my_program {
    use super::*;

    pub fn create_profile(ctx: Context<CreateProfile>, username: [u8; 32]) -> Result<()> {
        let profile = &mut ctx.accounts.profile;
        profile.owner = ctx.accounts.user.key();
        profile.username = username;
        profile.level = 1;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateProfile<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        space = 8 + UserProfile::SIZE,
        seeds = [b"profile", user.key().as_ref()],
        bump
    )]
    pub profile: Account<'info, UserProfile>,

    pub system_program: Program<'info, System>,

    // Light Protocol infrastructure (auto-detected)
    pub light_system_program: Program<'info, LightSystem>,
    pub account_compression_program: Program<'info, AccountCompression>,
}
```

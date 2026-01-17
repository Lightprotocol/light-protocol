# `#[rentfree_program]` Attribute Macro

## 1. Overview

The `#[rentfree_program]` attribute macro provides program-level auto-discovery and instruction wrapping for Light Protocol's rent-free compression system. When applied to an Anchor program module, it:

1. **Discovers** all `#[rentfree]` and `#[rentfree_token]` fields in `#[derive(Accounts)]` structs across the crate
2. **Auto-wraps** instruction handlers with `light_pre_init`/`light_finalize` lifecycle hooks
3. **Generates** compression/decompression instructions, variant enums, seed structs, and client helper functions

The macro reads external module files at compile time following Anchor's module resolution pattern, extracting seed information from `#[account(seeds = [...])]` attributes.

**Location**: `sdk-libs/macros/src/rentfree/program/`

## 2. Usage

### Basic Application

Apply `#[rentfree_program]` before `#[program]` on your Anchor program module:

```rust
use light_sdk_macros::rentfree_program;

#[rentfree_program]
#[program]
pub mod my_program {
    pub mod instruction_accounts;  // Macro reads this file
    pub mod state;

    use instruction_accounts::*;
    use state::*;

    // No #[light_instruction] needed - automatically wrapped!
    pub fn create_user(
        ctx: Context<CreateUser>,
        params: CreateUserParams,
    ) -> Result<()> {
        // Your business logic
        ctx.accounts.user.owner = params.owner;
        Ok(())
    }
}
```

### Required Attributes on Accounts Structs

In your instruction accounts module, use `#[rentfree]` for PDA accounts and `#[rentfree_token(authority = [...])]` for token accounts:

```rust
use anchor_lang::prelude::*;
use light_sdk_macros::RentFree;

#[derive(Accounts, RentFree)]
#[instruction(params: CreateUserParams)]
pub struct CreateUser<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub authority: Signer<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + UserRecord::INIT_SPACE,
        seeds = [
            b"user_record",
            authority.key().as_ref(),
            params.owner.as_ref(),
            params.category_id.to_le_bytes().as_ref()
        ],
        bump,
    )]
    #[rentfree]
    pub user_record: Account<'info, UserRecord>,

    #[account(
        mut,
        seeds = [b"vault", cmint.key().as_ref()],
        bump,
    )]
    #[rentfree_token(authority = [b"vault_authority"])]
    pub vault: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}
```

### Seed Expression Support

Seeds can reference:
- **Literals**: `b"seed"` or `"seed"`
- **Constants**: `MY_SEED` (uppercase identifiers resolved as `crate::MY_SEED`)
- **Context accounts**: `authority.key().as_ref()` -> extracted as `ctx.accounts.authority`
- **Instruction data**: `params.owner.as_ref()` or `params.category_id.to_le_bytes().as_ref()`
- **Function calls**: `max_key(&fee_payer.key(), &authority.key()).as_ref()`

## 3. Generated Items

### 3.1 RentFreeAccountVariant Enum

A unified enum representing all compressible account types in both packed (serialized) and unpacked forms:

```rust
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum RentFreeAccountVariant {
    // For each #[rentfree] account type
    UserRecord { data: UserRecord, authority: Pubkey },
    PackedUserRecord { data: PackedUserRecord, authority_idx: u8 },

    GameSession { data: GameSession, fee_payer: Pubkey, authority: Pubkey },
    PackedGameSession { data: PackedGameSession, fee_payer_idx: u8, authority_idx: u8 },

    // Token variants
    PackedCTokenData(PackedCTokenData<PackedTokenAccountVariant>),
    CTokenData(CTokenData<TokenAccountVariant>),
}
```

The enum implements:
- `light_hasher::DataHasher` - for computing compressed account hashes
- `light_sdk::LightDiscriminator` - discriminator for account identification
- `light_sdk::compressible::HasCompressionInfo` - compression metadata access
- `light_sdk::compressible::Pack/Unpack` - serialization with account index packing

### 3.2 Seeds Structs

For each PDA type, a seeds struct and constructor are generated:

```rust
#[derive(Clone, Debug)]
pub struct UserRecordSeeds {
    pub authority: Pubkey,  // from ctx.accounts.authority
    pub owner: Pubkey,      // from params.owner (data field)
    pub category_id: u64,   // from params.category_id (data field)
}

impl RentFreeAccountVariant {
    pub fn user_record(
        account_data: &[u8],
        seeds: UserRecordSeeds,
    ) -> Result<Self, anchor_lang::error::Error> {
        use anchor_lang::AnchorDeserialize;
        let data = UserRecord::deserialize(&mut &account_data[..])?;

        // Verify data fields match seeds
        if data.owner != seeds.owner {
            return Err(RentFreeInstructionError::SeedMismatch.into());
        }
        if data.category_id != seeds.category_id {
            return Err(RentFreeInstructionError::SeedMismatch.into());
        }

        Ok(Self::UserRecord {
            data,
            authority: seeds.authority,
        })
    }
}

impl IntoVariant<RentFreeAccountVariant> for UserRecordSeeds {
    fn into_variant(self, data: &[u8]) -> Result<RentFreeAccountVariant, Error> {
        RentFreeAccountVariant::user_record(data, self)
    }
}
```

### 3.3 CtxSeeds Structs

For PDA seed derivation during decompression, context seed structs hold resolved Pubkeys:

```rust
#[derive(Default)]
pub struct UserRecordCtxSeeds {
    pub authority: Pubkey,
}

impl PdaSeedDerivation<UserRecordCtxSeeds, ()> for UserRecord {
    fn derive_pda_seeds_with_accounts(
        &self,
        program_id: &Pubkey,
        ctx_seeds: &UserRecordCtxSeeds,
        _seed_params: &(),
    ) -> Result<(Vec<Vec<u8>>, Pubkey), ProgramError> {
        let seeds: &[&[u8]] = &[
            b"user_record",
            ctx_seeds.authority.as_ref(),
            self.owner.as_ref(),
            self.category_id.to_le_bytes().as_ref(),
        ];
        let (pda, bump) = Pubkey::find_program_address(seeds, program_id);
        let mut seeds_vec = Vec::with_capacity(seeds.len() + 1);
        // ... build seeds_vec with bump
        Ok((seeds_vec, pda))
    }
}
```

### 3.4 Decompress Instruction

The `decompress_accounts_idempotent` instruction recreates on-chain PDA accounts from compressed state:

```rust
#[derive(Accounts)]
pub struct DecompressAccountsIdempotent<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK: Checked by SDK
    pub config: AccountInfo<'info>,
    /// CHECK: anyone can pay
    #[account(mut)]
    pub rent_sponsor: UncheckedAccount<'info>,
    /// CHECK: optional - only needed if decompressing tokens
    #[account(mut)]
    pub ctoken_rent_sponsor: Option<AccountInfo<'info>>,
    /// CHECK:
    #[account(address = pubkey!("cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m"))]
    pub light_token_program: Option<UncheckedAccount<'info>>,
    /// CHECK:
    #[account(address = pubkey!("GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy"))]
    pub ctoken_cpi_authority: Option<UncheckedAccount<'info>>,
    /// CHECK: Checked by SDK
    pub ctoken_config: Option<UncheckedAccount<'info>>,
}

pub fn decompress_accounts_idempotent<'info>(
    ctx: Context<'_, '_, '_, 'info, DecompressAccountsIdempotent<'info>>,
    proof: ValidityProof,
    compressed_accounts: Vec<RentFreeAccountData>,
    system_accounts_offset: u8,
) -> Result<()> {
    // Delegates to process_decompress_accounts_idempotent
}
```

### 3.5 Compress Instruction

The `compress_accounts_idempotent` instruction compresses on-chain PDA accounts back to compressed state:

```rust
#[derive(Accounts)]
pub struct CompressAccountsIdempotent<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK: Checked by SDK
    pub config: AccountInfo<'info>,
    /// CHECK: Checked by SDK
    #[account(mut)]
    pub rent_sponsor: AccountInfo<'info>,
    /// CHECK: Checked by SDK
    #[account(mut)]
    pub compression_authority: AccountInfo<'info>,
}

pub fn compress_accounts_idempotent<'info>(
    ctx: Context<'_, '_, '_, 'info, CompressAccountsIdempotent<'info>>,
    proof: ValidityProof,
    compressed_accounts: Vec<CompressedAccountMetaNoLamportsNoAddress>,
    system_accounts_offset: u8,
) -> Result<()> {
    // Delegates to process_compress_accounts_idempotent
}
```

### 3.6 Config Instructions

Configuration management instructions for the compression system:

```rust
#[derive(Accounts)]
pub struct InitializeCompressionConfig<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: Checked by SDK
    #[account(mut)]
    pub config: AccountInfo<'info>,
    /// CHECK: Checked by SDK
    pub program_data: AccountInfo<'info>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn initialize_compression_config<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeCompressionConfig<'info>>,
    write_top_up: u32,
    rent_sponsor: Pubkey,
    compression_authority: Pubkey,
    rent_config: RentConfig,
    address_space: Vec<Pubkey>,
) -> Result<()>;

#[derive(Accounts)]
pub struct UpdateCompressionConfig<'info> {
    /// CHECK: Checked by SDK
    #[account(mut)]
    pub config: AccountInfo<'info>,
    pub update_authority: Signer<'info>,
}

pub fn update_compression_config<'info>(
    ctx: Context<'_, '_, '_, 'info, UpdateCompressionConfig<'info>>,
    new_rent_sponsor: Option<Pubkey>,
    new_compression_authority: Option<Pubkey>,
    new_rent_config: Option<RentConfig>,
    new_write_top_up: Option<u32>,
    new_address_space: Option<Vec<Pubkey>>,
    new_update_authority: Option<Pubkey>,
) -> Result<()>;
```

### 3.7 Client Seed Functions

Helper functions for deriving PDAs on the client side:

```rust
mod __client_seed_functions {
    use super::*;

    pub fn get_user_record_seeds(
        authority: &Pubkey,
        owner: &Pubkey,
        category_id: u64,
    ) -> (Vec<Vec<u8>>, Pubkey) {
        let mut seed_values = Vec::with_capacity(5);
        seed_values.push(b"user_record".to_vec());
        seed_values.push(authority.as_ref().to_vec());
        seed_values.push(owner.as_ref().to_vec());
        seed_values.push(category_id.to_le_bytes().to_vec());
        let seed_slices: Vec<&[u8]> = seed_values.iter().map(|v| v.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seed_slices, &crate::ID);
        seed_values.push(vec![bump]);
        (seed_values, pda)
    }

    // For token accounts
    pub fn get_vault_seeds(mint: &Pubkey) -> (Vec<Vec<u8>>, Pubkey) { ... }
    pub fn get_vault_authority_seeds(_program_id: &Pubkey) -> (Vec<Vec<u8>>, Pubkey) { ... }
}

pub use __client_seed_functions::*;
```

### 3.8 TokenAccountVariant Enum

For `#[rentfree_token]` fields, packed/unpacked token variant enums are generated:

```rust
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
pub enum TokenAccountVariant {
    Vault { mint: Pubkey },
    UserAta { owner: Pubkey },
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
pub enum PackedTokenAccountVariant {
    Vault { mint_idx: u8 },
    UserAta { owner_idx: u8 },
}

impl TokenSeedProvider for TokenAccountVariant {
    fn get_seeds(&self, program_id: &Pubkey) -> Result<(Vec<Vec<u8>>, Pubkey), ProgramError> {
        match self {
            TokenAccountVariant::Vault { mint } => {
                let seeds: &[&[u8]] = &[b"vault", mint.as_ref()];
                // ... derive PDA
            }
            // ...
        }
    }

    fn get_authority_seeds(&self, program_id: &Pubkey) -> Result<(Vec<Vec<u8>>, Pubkey), ProgramError> {
        // Returns authority PDA seeds for signing token operations
    }
}
```

### 3.9 Error Codes

```rust
#[error_code]
pub enum RentFreeInstructionError {
    #[msg("Rent sponsor mismatch")]
    InvalidRentSponsor,
    #[msg("Missing seed account")]
    MissingSeedAccount,
    #[msg("Seed value does not match account data")]
    SeedMismatch,
    #[msg("Not implemented")]
    CTokenDecompressionNotImplemented,
    #[msg("Not implemented")]
    PdaDecompressionNotImplemented,
    #[msg("Not implemented")]
    TokenCompressionNotImplemented,
    #[msg("Not implemented")]
    PdaCompressionNotImplemented,
}
```

## 4. Code Generation Flow

```
                    #[rentfree_program]
                           |
                           v
            +-----------------------------+
            |   rentfree_program_impl()   |
            |   (instructions.rs:389)     |
            +-----------------------------+
                           |
         +-----------------+-----------------+
         |                                   |
         v                                   v
+------------------+              +----------------------+
| CrateContext     |              | extract_context_and_ |
| ::parse_from_    |              | params() + wrap_     |
| manifest()       |              | function_with_       |
| (crate_context.rs)|              | rentfree()          |
+------------------+              | (parsing.rs)         |
         |                        +----------------------+
         v                                   |
+------------------+                         |
| structs_with_    |                         |
| derive("Accounts")|                        |
+------------------+                         |
         |                                   |
         v                                   |
+------------------------+                   |
| extract_from_accounts_ |                   |
| struct()               |                   |
| (seed_extraction.rs)   |                   |
+------------------------+                   |
         |                                   |
         v                                   v
+--------------------------------------------------+
|                    codegen()                      |
|                 (instructions.rs:37)              |
+--------------------------------------------------+
         |
         +---> validate_compressed_account_sizes()
         |                    (compress.rs)
         |
         +---> compressed_account_variant_with_ctx_seeds()
         |                    (variant_enum.rs)
         |
         +---> generate_ctoken_account_variant_enum()
         |                    (variant_enum.rs)
         |
         +---> generate_decompress_*()
         |                    (decompress.rs)
         |
         +---> generate_compress_*()
         |                    (compress.rs)
         |
         +---> generate_pda_seed_provider_impls()
         |                    (decompress.rs)
         |
         +---> generate_ctoken_seed_provider_implementation()
         |                    (seed_codegen.rs)
         |
         +---> generate_client_seed_functions()
                             (seed_codegen.rs)
```

## 5. Source Code Structure

```
sdk-libs/macros/src/rentfree/program/
|-- mod.rs                 # Module exports, main entry point rentfree_program_impl
|-- instructions.rs        # Main orchestration: codegen(), rentfree_program_impl()
|-- parsing.rs             # Core types (TokenSeedSpec, SeedElement, InstructionDataSpec)
|                          # Expression analysis, seed conversion, function wrapping
|-- compress.rs            # CompressAccountsIdempotent generation
|                          # CompressContext trait impl, compress processor
|-- decompress.rs          # DecompressAccountsIdempotent generation
|                          # DecompressContext trait impl, PDA seed provider impls
|-- variant_enum.rs        # RentFreeAccountVariant enum generation
|                          # TokenAccountVariant/PackedTokenAccountVariant generation
|                          # Pack/Unpack trait implementations
|-- seed_codegen.rs        # Client seed function generation
|                          # TokenSeedProvider implementation generation
|-- crate_context.rs       # Anchor-style crate parsing (CrateContext, ParsedModule)
|                          # Module file discovery and parsing
|-- expr_traversal.rs      # AST expression transformation (ctx.field -> ctx_seeds.field)
|-- seed_utils.rs          # Seed expression conversion utilities
|                          # SeedConversionConfig, seed_element_to_ref_expr()
|-- visitors.rs            # Visitor-based AST traversal (FieldExtractor)
|                          # ClientSeedInfo classification and code generation
```

### Related Files

```
sdk-libs/macros/src/rentfree/
|-- traits/
|   |-- seed_extraction.rs    # ClassifiedSeed enum, Anchor seed parsing
|   |                         # extract_from_accounts_struct()
|   |-- decompress_context.rs # DecompressContext trait impl generation
|   |-- utils.rs              # Shared utilities (is_pubkey_type, etc.)
|-- shared_utils.rs           # Cross-module utilities (is_constant_identifier, etc.)
```

## 6. Key Implementation Details

### Automatic Function Wrapping

Functions using `#[rentfree]` Accounts structs are automatically wrapped with lifecycle hooks:

```rust
// Original:
pub fn create_user(ctx: Context<CreateUser>, params: Params) -> Result<()> {
    ctx.accounts.user.owner = params.owner;
    Ok(())
}

// Wrapped (generated):
pub fn create_user(ctx: Context<CreateUser>, params: Params) -> Result<()> {
    use light_sdk::compressible::{LightPreInit, LightFinalize};

    // Phase 1: Pre-init (registers compressed addresses)
    let __has_pre_init = ctx.accounts.light_pre_init(ctx.remaining_accounts, &params)?;

    // Execute original handler
    let __light_handler_result = (|| {
        ctx.accounts.user.owner = params.owner;
        Ok(())
    })();

    // Phase 2: Finalize compression on success
    if __light_handler_result.is_ok() {
        ctx.accounts.light_finalize(ctx.remaining_accounts, &params, __has_pre_init)?;
    }

    __light_handler_result
}
```

### Size Validation

Compressed accounts are validated at compile time to not exceed 800 bytes:

```rust
const _: () = {
    const COMPRESSED_SIZE: usize = 8 + <UserRecord as CompressedInitSpace>::COMPRESSED_INIT_SPACE;
    if COMPRESSED_SIZE > 800 {
        panic!("Compressed account 'UserRecord' exceeds 800-byte compressible account size limit.");
    }
};
```

### Instruction Variants

The macro supports three instruction variants based on field types:
- `PdaOnly`: Only `#[rentfree]` PDA fields
- `TokenOnly`: Only `#[rentfree_token]` token fields
- `Mixed`: Both PDA and token fields (most common)

Currently, only `Mixed` variant is fully implemented. `PdaOnly` and `TokenOnly` will error at runtime.

---

## 7. Limitations

### Compressed Account Size
- Maximum compressed account size is **800 bytes** (discriminator + data)
- Accounts exceeding this limit will fail at compile time with a descriptive error

### Instruction Variant Support
- `Mixed` (PDA + token fields): Fully implemented
- `PdaOnly`: Returns `unreachable!()` at runtime (not yet implemented)
- `TokenOnly`: Returns `unreachable!()` at runtime (not yet implemented)

### Crate Discovery
- Requires `CARGO_MANIFEST_DIR` environment variable (set by cargo)
- Module files must follow Anchor's `pub mod name;` pattern for discovery
- Inline `mod name { }` blocks are not discovered

### Token Authority Requirement
- `#[rentfree_token]` fields must specify `authority = [...]` seeds
- Authority is required for compression signing operations

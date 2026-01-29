# light-sdk-macros

Procedural macros for Light Protocol's rent-free compression system.

## Summary

- Provides derive macros for rent-free compressed accounts on Solana with minimal boilerplate
- `#[derive(LightAccounts)]` generates `LightPreInit`/`LightFinalize` for Anchor Accounts structs
- `#[derive(LightAccount)]` generates unified trait for compressible data structs with pack/unpack and compression_info accessors
- `#[light_program]` auto-discovers Light accounts and wraps instruction handlers

**Package**: `light-sdk-macros`
**Location**: `sdk-libs/macros/`

## Used In

- **`sdk-libs/sdk/`** - Runtime SDK with `LightPreInit`, `LightFinalize` trait definitions
- **`sdk-tests/csdk-anchor-full-derived-test/`** - Full macro integration tests
- **Programs using Light Protocol** - Any Anchor program that implements compressible accounts

## Main Macros

| Macro | Type | Purpose |
|-------|------|---------|
| `#[derive(LightAccounts)]` | Derive | Generates `LightPreInit`/`LightFinalize` for Accounts structs |
| `#[derive(LightAccount)]` | Derive | Unified trait with pack/unpack, compression_info accessors, space check |
| `#[light_program]` | Attribute | Program-level auto-discovery and instruction generation |
| `#[derive(LightHasherSha)]` | Derive | SHA256 hashing via DataHasher + ToByteArray |
| `#[derive(LightDiscriminator)]` | Derive | Unique 8-byte discriminator |

## Documentation

Detailed macro documentation is in the `docs/` directory:

- **`docs/CLAUDE.md`** - Documentation structure and navigation guide
- **`docs/accounts/architecture.md`** - `#[derive(LightAccounts)]` architecture and code generation
- **`docs/accounts/pda.md`** - `#[light_account(init)]` for compressed PDAs
- **`docs/accounts/mint.md`** - `#[light_account(init, mint::...)]` for compressed mints
- **`docs/accounts/token.md`** - `#[light_account([init,] token::...)]` for token accounts
- **`docs/accounts/associated_token.md`** - `#[light_account([init,] associated_token::...)]` for ATAs
- **`docs/account/architecture.md`** - `#[derive(LightAccount)]` for data structs
- **`docs/light_program/`** - `#[light_program]` attribute macro (architecture.md + codegen.md)

## Source Structure

```
src/
├── lib.rs                     # Macro entry points and doc comments
├── light_pdas/                # LightAccounts macro system
│   ├── mod.rs                 # Module exports
│   ├── shared_utils.rs        # Common utilities (MetaExpr, type helpers)
│   ├── light_account_keywords.rs  # Keyword parsing for #[light_account(...)]
│   ├── account/               # Trait derive macros for account DATA structs
│   │   ├── light_compressible.rs  # LightAccount derive
│   │   ├── seed_extraction.rs # Anchor seed extraction from #[account(...)]
│   │   └── utils.rs           # Shared utilities
│   ├── accounts/              # #[derive(LightAccounts)] for ACCOUNTS structs
│   │   ├── derive.rs          # Main derive orchestration
│   │   ├── light_account.rs   # #[light_account(...)] attribute parsing
│   │   ├── builder.rs         # Code generation builder
│   │   ├── parse.rs           # Attribute parsing with darling
│   │   ├── pda.rs             # PDA block code generation
│   │   ├── mint.rs            # Mint action CPI generation
│   │   ├── token.rs           # Token account handling
│   │   └── variant.rs         # Variant enum generation
│   ├── program/               # #[light_program] attribute macro
│   │   ├── instructions.rs    # Instruction handler generation
│   │   ├── compress.rs        # Compress instruction codegen
│   │   ├── decompress.rs      # Decompress instruction codegen
│   │   ├── variant_enum.rs    # LightAccountVariant enum generation
│   │   ├── parsing.rs         # Module parsing
│   │   ├── visitors.rs        # AST visitors
│   │   └── seed_codegen.rs    # Seed struct code generation
│   └── seeds/                 # Seed extraction and classification
│       ├── extract.rs         # Anchor seed extraction
│       ├── classify.rs        # Seed type classification
│       └── types.rs           # Seed type definitions
├── hasher/                    # LightHasher/LightHasherSha derive macros
├── discriminator.rs           # LightDiscriminator derive macro
├── rent_sponsor.rs            # Rent sponsor PDA derivation macros
├── account.rs                 # #[account] attribute macro
└── utils.rs                   # General utilities
```

## Usage Example

```rust
use light_sdk_macros::{light_program, LightAccounts, LightAccount, LightDiscriminator, LightHasherSha};

// State account with compression support
#[derive(Default, Debug, InitSpace, LightAccount, LightDiscriminator, LightHasherSha)]
#[account]
pub struct UserRecord {
    pub compression_info: CompressionInfo,  // Non-Option, first or last field
    pub owner: Pubkey,
    pub score: u64,
}

// Accounts struct with rent-free field
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateParams)]
pub struct Create<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    #[account(init, payer = fee_payer, space = 8 + UserRecord::INIT_SPACE, seeds = [b"user", params.owner.as_ref()], bump)]
    #[light_account(init)]
    pub user_record: Account<'info, UserRecord>,
}

// Program with auto-wrapped instructions
#[light_program]
#[program]
pub mod my_program {
    pub fn create(ctx: Context<Create>, params: CreateParams) -> Result<()> {
        ctx.accounts.user_record.owner = params.owner;
        Ok(())
    }
}
```

## Requirements

Programs using these macros must define:
- `LIGHT_CPI_SIGNER: Pubkey` - CPI signer constant
- `ID` - Program ID (from `declare_id!`)

## Testing

```bash
cargo test -p light-sdk-macros
```

Integration tests are in `sdk-tests/`:
- `csdk-anchor-full-derived-test` - Full macro integration test

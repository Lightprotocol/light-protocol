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
│   │   ├── derive.rs          # LightAccount derive
│   │   ├── traits.rs          # Trait implementations
│   │   ├── utils.rs           # Shared utilities
│   │   └── validation.rs      # Account validation
│   ├── accounts/              # #[derive(LightAccounts)] for ACCOUNTS structs
│   │   ├── derive.rs          # Main derive orchestration
│   │   ├── light_account.rs   # #[light_account(...)] attribute parsing
│   │   ├── builder.rs         # Code generation builder
│   │   ├── parse.rs           # Attribute parsing with darling
│   │   ├── pda.rs             # PDA block code generation
│   │   ├── mint.rs            # Mint action CPI generation
│   │   ├── token.rs           # Token account handling
│   │   ├── validation.rs      # Accounts validation
│   │   └── variant.rs         # Variant enum generation
│   ├── parsing/               # Unified parsing infrastructure
│   │   ├── accounts_struct.rs # ParsedAccountsStruct for unified parsing
│   │   ├── crate_context.rs   # Crate-wide module parsing for struct discovery
│   │   ├── infra.rs           # Infrastructure field classification
│   │   └── instruction_arg.rs # Instruction argument parsing from #[instruction(...)]
│   ├── program/               # #[light_program] attribute macro
│   │   ├── instructions.rs    # Instruction handler generation
│   │   ├── compress.rs        # Compress instruction codegen
│   │   ├── decompress.rs      # Decompress instruction codegen
│   │   ├── variant_enum.rs    # LightAccountVariant enum generation
│   │   ├── parsing.rs         # Seed conversion and function wrapping
│   │   ├── visitors.rs        # AST visitors for field extraction
│   │   ├── seed_codegen.rs    # Seed struct code generation
│   │   ├── seed_utils.rs      # Seed utility functions
│   │   └── expr_traversal.rs  # Expression traversal utilities
│   └── seeds/                 # Seed extraction and classification
│       ├── anchor_extraction.rs # Extract seeds from #[account(seeds=[...])]
│       ├── classification.rs  # Seed type classification logic
│       ├── data_fields.rs     # Data field extraction from seeds
│       ├── extract.rs         # Main extraction from Accounts structs
│       ├── instruction_args.rs # InstructionArgSet type definition
│       └── types.rs           # ClassifiedSeed, SeedSpec type definitions
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

# light-sdk-macros

Procedural macros for Light Protocol's rent-free compression system.

## Crate Overview

This crate provides macros that enable rent-free compressed accounts on Solana with minimal boilerplate.

**Package**: `light-sdk-macros`
**Location**: `sdk-libs/macros/`

## Main Macros

| Macro | Type | Purpose |
|-------|------|---------|
| `#[derive(RentFree)]` | Derive | Generates `LightPreInit`/`LightFinalize` for Accounts structs |
| `#[rentfree_program]` | Attribute | Program-level auto-discovery and instruction generation |
| `#[derive(LightCompressible)]` | Derive | Combined traits for compressible account data |
| `#[derive(Compressible)]` | Derive | Compression traits (HasCompressionInfo, CompressAs, Size) |
| `#[derive(CompressiblePack)]` | Derive | Pack/Unpack with Pubkey-to-index compression |

## Documentation

Detailed macro documentation is in the `docs/` directory:

- **`docs/CLAUDE.md`** - Documentation structure guide
- **`docs/rentfree.md`** - `#[derive(RentFree)]` and trait derives
- **`docs/rentfree_program/`** - `#[rentfree_program]` attribute macro (architecture.md + codegen.md)

## Source Structure

```
src/
├── lib.rs                 # Macro entry points
├── rentfree/              # RentFree macro system
│   ├── account/           # Trait derive macros for account data structs
│   ├── accounts/          # #[derive(RentFree)] for Accounts structs
│   ├── program/           # #[rentfree_program] attribute macro
│   └── shared_utils.rs    # Common utilities
└── hasher/                # LightHasherSha derive macro
```

## Usage Example

```rust
use light_sdk_macros::{rentfree_program, RentFree, LightCompressible};

// State account with compression support
#[derive(Default, Debug, InitSpace, LightCompressible)]
#[account]
pub struct UserRecord {
    pub owner: Pubkey,
    pub score: u64,
    pub compression_info: Option<CompressionInfo>,
}

// Accounts struct with rent-free field
#[derive(Accounts, RentFree)]
#[instruction(params: CreateParams)]
pub struct Create<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    #[account(init, payer = fee_payer, space = 8 + UserRecord::INIT_SPACE, seeds = [b"user", params.owner.as_ref()], bump)]
    #[rentfree]
    pub user_record: Account<'info, UserRecord>,
}

// Program with auto-wrapped instructions
#[rentfree_program]
#[program]
pub mod my_program {
    pub fn create(ctx: Context<Create>, params: CreateParams) -> Result<()> {
        // Business logic - compression handled automatically
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

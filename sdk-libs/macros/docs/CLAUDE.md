# Documentation Structure

## Overview

Documentation for the Light PDA macro system in `light-sdk-macros`. These macros enable rent-free compressed accounts on Solana with minimal boilerplate.

## Structure

| File | Description |
|------|-------------|
| **`CLAUDE.md`** | This file - documentation structure guide |
| **`../CLAUDE.md`** | Main entry point for sdk-libs/macros |
| **`accounts/architecture.md`** | `#[derive(LightAccounts)]` architecture and code generation |
| **`accounts/pda.md`** | Compressed PDAs: usage, lifecycle, validations |
| **`accounts/mint.md`** | `#[light_account(init, mint::...)]` for compressed mints |
| **`accounts/token.md`** | `#[light_account([init,] token::...)]` for token accounts |
| **`accounts/associated_token.md`** | `#[light_account([init,] associated_token::...)]` for ATAs |
| **`light_program/`** | `#[light_program]` attribute macro |
| **`light_program/architecture.md`** | Architecture overview, usage, generated items |
| **`light_program/codegen.md`** | Technical implementation details (code generation) |
| **`account/architecture.md`** | `#[derive(LightAccount)]` for data structs |

### Accounts Field Attributes

Field-level attributes applied inside `#[derive(LightAccounts)]` Accounts structs. Each account type has dedicated documentation:

| File | Namespace | Description |
|------|-----------|-------------|
| **`accounts/pda.md`** | (none) | Compressed PDAs: usage, lifecycle, validations |
| **`accounts/mint.md`** | `mint::` | Compressed mints with optional TokenMetadata extension |
| **`accounts/token.md`** | `token::` | PDA-owned token accounts (vaults) |
| **`accounts/associated_token.md`** | `associated_token::` | User associated token accounts |

See `accounts/architecture.md` for shared infrastructure requirements, validation rules, and direct proof argument support.

### Account Data Struct Derives

| Macro | Description | Documentation |
|-------|-------------|---------------|
| `#[derive(LightAccount)]` | Unified trait: pack/unpack, compression_info accessors, space check | `account/architecture.md` |
| `#[derive(LightDiscriminator)]` | Unique 8-byte discriminator | - |
| `#[derive(LightHasherSha)]` | SHA256 hashing via DataHasher + ToByteArray | - |

## Navigation Tips

### Starting Points

- **Data structs**: Use `LightAccount` + `LightDiscriminator` + `LightHasherSha` derives with non-Option `CompressionInfo`
  - See `account/architecture.md` for details
  - `compression_info` field must be first or last field in struct
  - `INIT_SPACE` must be <= 800 bytes (enforced at compile time)
- **Accounts structs**: Use `accounts/architecture.md` for the accounts-level derive macro that marks fields for compression
  - Add `#[derive(LightAccounts)]` to Anchor `#[derive(Accounts)]` structs
  - Mark PDA fields with `#[light_account(init)]`
  - Mark mint fields with `#[light_account(init, mint::...)]`
  - Mark token fields with `#[light_account(token::...)]`
- **Program-level integration**: Use `light_program/architecture.md` for program-level auto-discovery and instruction generation
  - Add `#[light_program]` attribute above `#[program]`
  - Automatically discovers all `#[derive(LightAccounts)]` structs in the crate
  - Generates `LightAccountVariant` enum, seeds structs, compress/decompress instructions
- **Implementation details**: Use `light_program/codegen.md` for technical code generation details

### Finding Source Files

When debugging macro-generated code:
1. Use `cargo expand` to see the generated code (see root CLAUDE.md for details)
2. Search in `src/light_pdas/` for the relevant module:
   - `account/` - Data struct derives (LightAccount, etc.)
   - `accounts/` - Accounts struct derives (LightAccounts)
   - `program/` - Program-level macro (#[light_program])
   - `parsing/` - Shared parsing infrastructure
   - `seeds/` - Seed extraction and classification
3. Use `ast-grep` to understand code dependencies (see root CLAUDE.md)

### Macro Hierarchy

```
#[light_program]                      <- Program-level (light_program/)
    |
    +-- Discovers #[derive(LightAccounts)] structs
    |
    +-- Generates:
        - LightAccountVariant enum
        - Seeds structs
        - Compress/Decompress instructions
        - Config instructions

#[derive(LightAccounts)]              <- Accounts-level (accounts/architecture.md)
    |
    +-- Generates LightPreInit + LightFinalize impls
    |
    +-- Uses trait derives on data structs:
        - LightAccount                <- account/architecture.md
        - LightDiscriminator          <- discriminator.rs
        - LightHasherSha              <- hasher/
```

## Related Source Code

```
sdk-libs/macros/src/light_pdas/
├── account/             # Trait derive macros for account DATA structs
│   ├── derive.rs        # LightAccount derive
│   ├── traits.rs        # Trait implementations (HasCompressionInfo, CompressAs, Compressible)
│   ├── utils.rs         # Shared utilities
│   └── validation.rs    # Account validation
├── accounts/            # #[derive(LightAccounts)] for ACCOUNTS structs
│   ├── derive.rs        # Main derive orchestration
│   ├── light_account.rs # #[light_account(...)] parsing
│   ├── builder.rs       # Code generation builder
│   ├── parse.rs         # Attribute parsing with darling
│   ├── pda.rs           # PDA block code generation
│   ├── mint.rs          # Mint action CPI generation
│   ├── token.rs         # Token account handling
│   ├── validation.rs    # Accounts validation
│   └── variant.rs       # Variant enum generation
├── program/             # #[light_program] implementation
│   ├── mod.rs           # light_program_impl entry point
│   ├── instructions.rs  # Instruction handler wrapping
│   ├── compress.rs      # Compress instruction codegen
│   ├── decompress.rs    # Decompress instruction codegen
│   ├── variant_enum.rs  # LightAccountVariant enum generation
│   ├── parsing.rs       # Seed conversion and function wrapping
│   ├── visitors.rs      # AST visitors for field extraction
│   ├── seed_codegen.rs  # Seed struct code generation
│   ├── seed_utils.rs    # Seed utility functions
│   └── expr_traversal.rs # Expression traversal utilities
├── parsing/             # Unified parsing infrastructure
│   ├── accounts_struct.rs # ParsedAccountsStruct for unified parsing
│   ├── crate_context.rs   # Crate-wide module parsing for struct discovery
│   ├── infra.rs           # Infrastructure field classification
│   └── instruction_arg.rs # Instruction argument parsing from #[instruction(...)]
├── seeds/               # Seed extraction and classification
│   ├── extract.rs       # Main extraction from Accounts structs
│   ├── anchor_extraction.rs # Extract seeds from #[account(seeds=[...])]
│   ├── classification.rs # Seed type classification logic
│   ├── data_fields.rs   # Data field extraction from seeds
│   ├── instruction_args.rs # InstructionArgSet type definition
│   └── types.rs         # ClassifiedSeed, ExtractedSeedSpec type definitions
├── light_account_keywords.rs # Keyword parsing for #[light_account(...)]
├── shared_utils.rs      # Common utilities (MetaExpr, type helpers)
└── mod.rs               # Module exports
```

# Documentation Structure

## Overview

Documentation for the Light PDA macro system in `light-sdk-macros`. These macros enable rent-free compressed accounts on Solana with minimal boilerplate.

## Structure

| File | Description |
|------|-------------|
| **`CLAUDE.md`** | This file - documentation structure guide |
| **`../CLAUDE.md`** | Main entry point for sdk-libs/macros |
| **`accounts/architecture.md`** | `#[derive(LightAccounts)]` architecture and code generation |
| **`accounts/pda.md`** | `#[light_account(init)]` for compressed PDAs |
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
| **`accounts/pda.md`** | (none) | Compressed PDAs with `#[light_account(init)]` |
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
- **Accounts structs**: Use `accounts/architecture.md` for the accounts-level derive macro that marks fields for compression
- **Program-level integration**: Use `light_program/architecture.md` for program-level auto-discovery and instruction generation
- **Implementation details**: Use `light_program/codegen.md` for technical code generation details

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
│   ├── light_compressible.rs  # LightAccount derive
│   ├── seed_extraction.rs     # Anchor seed extraction
│   └── utils.rs               # Shared utilities
├── accounts/            # #[derive(LightAccounts)] for ACCOUNTS structs
│   ├── derive.rs        # Main derive orchestration
│   ├── light_account.rs # #[light_account(...)] parsing
│   ├── builder.rs       # Code generation builder
│   ├── parse.rs         # Attribute parsing
│   ├── pda.rs           # PDA code generation
│   ├── mint.rs          # Mint code generation
│   └── token.rs         # Token/ATA code generation
├── program/             # #[light_program] implementation
│   ├── instructions.rs  # Instruction handler generation
│   ├── compress.rs      # Compress instruction codegen
│   ├── decompress.rs    # Decompress instruction codegen
│   └── variant_enum.rs  # LightAccountVariant enum generation
├── seeds/               # Seed extraction and classification
├── shared_utils.rs      # Common utilities
└── mod.rs               # Module exports
```

# Rent-Free Macros

Procedural macros for generating rent-free account types and their hooks for Solana programs.

## Directory Structure

```
rentfree/
├── mod.rs              # Module declaration
├── README.md           # This file
├── accounts/           # #[derive(LightAccounts)] implementation
│   ├── mod.rs          # Entry point: derive_rentfree()
│   ├── parse.rs        # Parsing #[light_account(init)], #[light_account(init)] attributes
│   └── codegen.rs      # LightPreInit/LightFinalize trait generation
├── program/            # #[rentfree_program] implementation
│   ├── mod.rs          # Entry point: rentfree_program_impl()
│   ├── instructions.rs # Instruction generation and handler wrapping
│   ├── crate_context.rs # Crate scanning for #[derive(Accounts)] structs
│   ├── variant_enum.rs # RentFreeAccountVariant enum generation
│   └── seed_providers.rs # PDA/CToken seed derivation implementations
└── traits/             # Shared trait derive macros
    ├── mod.rs          # Module declaration
    ├── traits.rs       # HasCompressionInfo, CompressAs, Compressible
    ├── pack_unpack.rs  # Pack/Unpack trait implementations
    ├── light_compressible.rs # LightAccount combined derive
    ├── anchor_seeds.rs # Seed extraction from Anchor attributes
    ├── decompress_context.rs # DecompressContext trait generation
    └── utils.rs        # Shared utility functions
```

## Modules

### `accounts/` - RentFree Derive Macro

Implements `#[derive(LightAccounts)]` for Anchor Accounts structs:

- **parse.rs** - Parses `#[light_account(init)]`, `#[light_account(token)]`, `#[light_account(init)]` attributes
- **codegen.rs** - Generates `LightPreInit` and `LightFinalize` trait implementations

### `program/` - RentFree Program Macro

Implements `#[rentfree_program]` attribute macro:

- **instructions.rs** - Main macro logic, generates compress/decompress handlers
- **crate_context.rs** - Scans crate for `#[derive(Accounts)]` structs
- **variant_enum.rs** - Generates `RentFreeAccountVariant` enum with all traits
- **seed_providers.rs** - PDA and CToken seed provider implementations

### `traits/` - Shared Trait Derives

Core trait implementations shared across macros:

- **traits.rs** - `HasCompressionInfo`, `CompressAs`, `Compressible` derives
- **pack_unpack.rs** - Generates `PackedXxx` structs, `Pack`/`Unpack` traits
- **light_compressible.rs** - `LightAccount` combined derive macro
- **anchor_seeds.rs** - Extracts seeds from `#[account(seeds = [...])]`
- **decompress_context.rs** - `DecompressContext` trait generation
- **utils.rs** - Shared utilities (e.g., empty CToken enum generation)

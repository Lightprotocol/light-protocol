# Documentation Structure

## Overview

Documentation for the rentfree macro system in `light-sdk-macros`. These macros enable rent-free compressed accounts on Solana with minimal boilerplate.

## Structure

| File | Description |
|------|-------------|
| **`CLAUDE.md`** | This file - documentation structure guide |
| **`../CLAUDE.md`** | Main entry point for sdk-libs/macros |
| **`rentfree.md`** | `#[derive(RentFree)]` macro and trait derives |
| **`rentfree_program/`** | `#[rentfree_program]` attribute macro |
| **`rentfree_program/architecture.md`** | Architecture overview, usage, generated items |
| **`rentfree_program/codegen.md`** | Technical implementation details (code generation) |
| **`accounts/`** | Field-level attributes for Accounts structs |
| **`account/`** | Trait derive macros for account data structs |

### Accounts Field Attributes

Field-level attributes applied inside `#[derive(RentFree)]` Accounts structs:

| File | Attribute | Description |
|------|-----------|-------------|
| **`accounts/light_mint.md`** | `#[light_mint(...)]` | Creates compressed mint with automatic decompression |

See also: `#[rentfree]` attribute documented in `rentfree.md`

### Account Trait Documentation

| File | Macro | Description |
|------|-------|-------------|
| **`account/has_compression_info.md`** | `#[derive(HasCompressionInfo)]` | Accessor methods for compression_info field |
| **`account/compress_as.md`** | `#[derive(CompressAs)]` | Creates compressed representation for hashing |
| **`account/compressible.md`** | `#[derive(Compressible)]` | Combined: HasCompressionInfo + CompressAs + Size |
| **`account/compressible_pack.md`** | `#[derive(CompressiblePack)]` | Pack/Unpack with Pubkey-to-index compression |
| **`account/light_compressible.md`** | `#[derive(LightCompressible)]` | All traits for rent-free accounts |

## Navigation Tips

### Starting Points

- **Data struct traits**: Start with `account/light_compressible.md` for the all-in-one derive macro for compressible data structs
- **Building account structs**: Use `rentfree.md` for the accounts-level derive macro that marks fields for compression
- **Program-level integration**: Use `rentfree_program/architecture.md` for program-level auto-discovery and instruction generation
- **Implementation details**: Use `rentfree_program/codegen.md` for technical code generation details

### Macro Hierarchy

```
#[rentfree_program]          <- Program-level (rentfree_program/)
    |
    +-- Discovers #[derive(RentFree)] structs
    |
    +-- Generates:
        - RentFreeAccountVariant enum
        - Seeds structs
        - Compress/Decompress instructions
        - Config instructions

#[derive(RentFree)]          <- Account-level (rentfree.md)
    |
    +-- Generates LightPreInit + LightFinalize impls
    |
    +-- Uses trait derives (account/):
        - HasCompressionInfo      <- account/has_compression_info.md
        - CompressAs              <- account/compress_as.md
        - Compressible            <- account/compressible.md
        - CompressiblePack        <- account/compressible_pack.md
        - LightCompressible       <- account/light_compressible.md (combines all)
```

## Related Source Code

```
sdk-libs/macros/src/rentfree/
├── account/         # Trait derive macros for account data structs
├── accounts/        # #[derive(RentFree)] implementation
├── program/         # #[rentfree_program] implementation
├── shared_utils.rs  # Common utilities
└── mod.rs           # Module exports
```

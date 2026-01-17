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
| **`traits/`** | Trait derive macros for compressible data structs |

### Traits Documentation

| File | Macro | Description |
|------|-------|-------------|
| **`traits/has_compression_info.md`** | `#[derive(HasCompressionInfo)]` | Accessor methods for compression_info field |
| **`traits/compress_as.md`** | `#[derive(CompressAs)]` | Creates compressed representation for hashing |
| **`traits/compressible.md`** | `#[derive(Compressible)]` | Combined: HasCompressionInfo + CompressAs + Size |
| **`traits/compressible_pack.md`** | `#[derive(CompressiblePack)]` | Pack/Unpack with Pubkey-to-index compression |
| **`traits/light_compressible.md`** | `#[derive(LightCompressible)]` | All traits for rent-free accounts |

## Navigation Tips

### Starting Points

- **Data struct traits**: Start with `traits/light_compressible.md` for the all-in-one derive macro for compressible data structs
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
    +-- Uses trait derives (traits/):
        - HasCompressionInfo      <- traits/has_compression_info.md
        - CompressAs              <- traits/compress_as.md
        - Compressible            <- traits/compressible.md
        - CompressiblePack        <- traits/compressible_pack.md
        - LightCompressible       <- traits/light_compressible.md (combines all)
```

## Related Source Code

```
sdk-libs/macros/src/rentfree/
├── accounts/        # #[derive(RentFree)] implementation
├── program/         # #[rentfree_program] implementation
├── traits/          # Trait derive macros
├── shared_utils.rs  # Common utilities
└── mod.rs           # Module exports
```

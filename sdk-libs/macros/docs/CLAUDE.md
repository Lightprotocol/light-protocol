# Documentation Structure

## Overview

Documentation for the rentfree macro system in `light-sdk-macros`. These macros enable rent-free compressed accounts on Solana with minimal boilerplate.

## Structure

| File | Description |
|------|-------------|
| **`CLAUDE.md`** | This file - documentation structure guide |
| **`../CLAUDE.md`** | Main entry point for sdk-libs/macros |
| **`rentfree.md`** | `#[derive(RentFree)]` macro and trait derives |
| **`rentfree_program.md`** | `#[rentfree_program]` attribute macro |

## Navigation Tips

### Starting Points

- **Building account structs**: Start with `rentfree.md` for the accounts-level derive macro that marks fields for compression
- **Program-level integration**: Use `rentfree_program.md` for program-level auto-discovery and instruction generation

### Macro Hierarchy

```
#[rentfree_program]          <- Program-level (rentfree_program.md)
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
    +-- Uses trait derives:
        - HasCompressionInfo
        - Compressible
        - Pack/Unpack
        - LightCompressible
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

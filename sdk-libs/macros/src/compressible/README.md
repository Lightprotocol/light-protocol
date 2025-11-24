# Compressible Macros

Procedural macros for generating rent-free account types and their hooks for Solana programs.

## Files

**`mod.rs`** - Module declaration

**`traits.rs`** - Core trait implementations

- `HasCompressionInfo` - CompressionInfo field access
- `CompressAs` - Field-level compression control
- `Compressible` - Full trait bundle (Size + HasCompressionInfo + CompressAs)

**`pack_unpack.rs`** - Pubkey compression

- Generates `PackedXxx` structs where Pubkey fields become u8 indices
- Implements Pack/Unpack traits for serialization efficiency

**`variant_enum.rs`** - Account variant enum

- Generates `CompressedAccountVariant` enum from account types
- Implements all required traits (Default, DataHasher, Size, Pack, Unpack)
- Creates `CompressedAccountData` wrapper struct

**`instructions.rs`** - Instruction generation

- Main macro: `add_compressible_instructions`
- Generates compress/decompress instruction handlers
- Creates context structs and account validation
- **Compress**: PDA-only (ctokens compressed via registry)
- **Decompress**: Full PDA + ctoken support

**`seed_providers.rs`** - Seed derivation

- PDA seed provider implementations
- CToken seed provider with account/authority derivation
- Client-side seed functions for off-chain use

**`decompress_context.rs`** - Decompression trait

- Generates `DecompressContext` implementation
- Account accessor methods
- PDA/token separation logic
- Token processing delegation

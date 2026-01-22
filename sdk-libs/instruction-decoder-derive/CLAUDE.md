# light-instruction-decoder-derive

Procedural macros for generating `InstructionDecoder` implementations.

## Overview

This crate provides two macros for generating instruction decoders:

| Macro | Type | Purpose |
|-------|------|---------|
| `#[derive(InstructionDecoder)]` | Derive | Generate decoder for instruction enums |
| `#[instruction_decoder]` | Attribute | Auto-generate from Anchor program modules |

## Module Structure

```
src/
├── lib.rs              # Macro entry points only (~100 lines)
├── utils.rs            # Case conversion, discriminator, error handling
├── parsing.rs          # Darling-based attribute parsing structs
├── builder.rs          # InstructionDecoderBuilder (code generation)
├── derive_impl.rs      # #[derive(InstructionDecoder)] implementation
├── attribute_impl.rs   # #[instruction_decoder] attribute implementation
└── crate_context.rs    # Recursive crate parsing for Accounts struct discovery
```

## Key Features

### Multiple Discriminator Sizes

- **1 byte**: Native programs with simple instruction indices
- **4 bytes**: System-style programs (little-endian u32)
- **8 bytes**: Anchor programs (SHA256 prefix, default)

### Explicit Discriminators

Two syntax forms for specifying explicit discriminators:

1. **Integer**: `#[discriminator = 5]` - for 1-byte and 4-byte modes
2. **Array**: `#[discriminator(26, 16, 169, 7, 21, 202, 242, 25)]` - for 8-byte mode with custom discriminators

### Account Names Extraction

Two ways to specify account names:

1. **Accounts type reference**: `accounts = MyAccountsStruct` - extracts field names at compile time
2. **Inline names**: Direct array `["source", "dest", "authority"]`

When using `accounts = SomeType`, the macro uses `CrateContext` to parse the crate at macro expansion time and extract field names from the struct definition. This works for any struct with named fields (including standard Anchor `#[derive(Accounts)]` structs) without requiring any special trait implementation.

### Off-chain Only

All generated code is gated with `#[cfg(not(target_os = "solana"))]` since instruction decoding is only needed for logging/debugging.

## Usage Examples

### Derive Macro

```rust
use light_instruction_decoder_derive::InstructionDecoder;

#[derive(InstructionDecoder)]
#[instruction_decoder(
    program_id = "MyProgramId111111111111111111111111111111111",
    program_name = "My Program",      // optional
    discriminator_size = 8            // optional: 1, 4, or 8
)]
pub enum MyInstruction {
    // Reference Accounts struct for account names
    #[instruction_decoder(accounts = CreateRecord, params = CreateRecordParams)]
    CreateRecord,

    // Inline account names
    #[instruction_decoder(account_names = ["source", "dest"])]
    Transfer,

    // Explicit integer discriminator (for 1-byte or 4-byte modes)
    #[discriminator = 5]
    Close,

    // Explicit array discriminator (for 8-byte mode with custom discriminators)
    #[discriminator(26, 16, 169, 7, 21, 202, 242, 25)]
    #[instruction_decoder(account_names = ["fee_payer", "authority"])]
    CustomInstruction,
}
```

### Attribute Macro (Anchor Programs)

```rust
use light_instruction_decoder_derive::instruction_decoder;

#[instruction_decoder]  // or #[instruction_decoder(program_id = crate::ID)]
#[program]
pub mod my_program {
    pub fn create_record(ctx: Context<CreateRecord>, params: CreateParams) -> Result<()> { ... }
    pub fn transfer(ctx: Context<Transfer>) -> Result<()> { ... }
}
```

This generates `MyProgramInstructionDecoder` that:
- Gets program_id from `crate::ID` (or `declare_id!` if found)
- Extracts function names and converts to discriminators
- Discovers Accounts struct field names from the crate
- Decodes params using borsh if specified

## Architecture

### Darling-Based Parsing

Attributes are parsed using the `darling` crate for:
- Declarative struct-based definitions
- Automatic validation
- Better error messages with span preservation

### Builder Pattern

`InstructionDecoderBuilder` separates:
- **Parsing**: Extract and validate attributes
- **Code Generation**: Produce TokenStream output

This follows the pattern from `sdk-libs/macros`.

### Crate Context

`CrateContext` recursively parses all module files at macro expansion time to discover structs by name. This enables both macros to automatically find field names:

- **Derive macro**: When `accounts = SomeType` is specified, extracts struct field names
- **Attribute macro**: Discovers Accounts structs from `Context<T>` parameters

The struct lookup finds any struct with named fields - no special trait implementation required. This makes the macro completely independent and works with any Anchor program.

## Testing

```bash
# Unit tests
cargo test -p light-instruction-decoder-derive

# Integration tests (verifies generated code compiles and works)
cargo test-sbf -p csdk-anchor-full-derived-test --test instruction_decoder_test
```

## Dependencies

- `darling`: Attribute parsing
- `syn/quote/proc-macro2`: Token manipulation
- `sha2`: Anchor discriminator computation
- `bs58`: Program ID encoding

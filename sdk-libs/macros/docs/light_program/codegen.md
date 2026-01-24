# `#[light_program]` Code Generation

Technical implementation details for the `#[light_program]` attribute macro.

## 1. Source Code Structure

```
sdk-libs/macros/src/rentfree/program/
|-- mod.rs                 # Module exports, main entry point light_program_impl
|-- instructions.rs        # Main orchestration: codegen(), light_program_impl()
|-- parsing.rs             # Core types (TokenSeedSpec, SeedElement, InstructionDataSpec)
|                          # Expression analysis, seed conversion, function wrapping
|-- compress.rs            # CompressAccountsIdempotent generation
|                          # CompressContext trait impl, compress processor
|-- decompress.rs          # DecompressAccountsIdempotent generation
|                          # DecompressContext trait impl, PDA seed provider impls
|-- variant_enum.rs        # LightAccountVariant enum generation
|                          # TokenAccountVariant/PackedTokenAccountVariant generation
|                          # Pack/Unpack trait implementations
|-- seed_codegen.rs        # Client seed function generation
|                          # TokenSeedProvider implementation generation
|-- crate_context.rs       # Anchor-style crate parsing (CrateContext, ParsedModule)
|                          # Module file discovery and parsing
|-- expr_traversal.rs      # AST expression transformation (ctx.field -> ctx_seeds.field)
|-- seed_utils.rs          # Seed expression conversion utilities
|                          # SeedConversionConfig, seed_element_to_ref_expr()
|-- visitors.rs            # Visitor-based AST traversal (FieldExtractor)
|                          # ClientSeedInfo classification and code generation
```

### Related Files

```
sdk-libs/macros/src/rentfree/
|-- traits/
|   |-- seed_extraction.rs    # ClassifiedSeed enum, Anchor seed parsing
|   |                         # extract_from_accounts_struct()
|   |-- decompress_context.rs # DecompressContext trait impl generation
|   |-- utils.rs              # Shared utilities (is_pubkey_type, etc.)
|-- shared_utils.rs           # Cross-module utilities (is_constant_identifier, etc.)
```


## 2. Code Generation Flow

```
                    #[light_program]
                           |
                           v
            +-----------------------------+
            |   light_program_impl()   |
            |   (instructions.rs:405)     |
            +-----------------------------+
                           |
         +-----------------+-----------------+
         |                                   |
         v                                   v
+------------------+              +----------------------+
| CrateContext     |              | extract_context_and_ |
| ::parse_from_    |              | params() + wrap_     |
| manifest()       |              | function_with_       |
| (crate_context.rs)|              | rentfree()          |
+------------------+              | (parsing.rs)         |
         |                        +----------------------+
         v                                   |
+------------------+                         |
| structs_with_    |                         |
| derive("Accounts")|                        |
+------------------+                         |
         |                                   |
         v                                   |
+------------------------+                   |
| extract_from_accounts_ |                   |
| struct()               |                   |
| (seed_extraction.rs)   |                   |
+------------------------+                   |
         |                                   |
         v                                   v
+--------------------------------------------------+
|                    codegen()                      |
|                 (instructions.rs:38)              |
+--------------------------------------------------+
         |
         +---> validate_compressed_account_sizes()
         |                    (compress.rs)
         |
         +---> compressed_account_variant_with_ctx_seeds()
         |                    (variant_enum.rs)
         |
         +---> generate_ctoken_account_variant_enum()
         |                    (variant_enum.rs)
         |
         +---> generate_decompress_*()
         |                    (decompress.rs)
         |
         +---> generate_compress_*()
         |                    (compress.rs)
         |
         +---> generate_pda_seed_provider_impls()
         |                    (decompress.rs)
         |
         +---> generate_ctoken_seed_provider_implementation()
         |                    (seed_codegen.rs)
         |
         +---> generate_client_seed_functions()
                             (seed_codegen.rs)
```


## 3. Key Implementation Details

### Automatic Function Wrapping

Functions using `#[light_account(init)]` Accounts structs are automatically wrapped with lifecycle hooks:

```rust
// Original:
pub fn create_user(ctx: Context<CreateUser>, params: Params) -> Result<()> {
    ctx.accounts.user.owner = params.owner;
    Ok(())
}

// Wrapped (generated):
pub fn create_user(ctx: Context<CreateUser>, params: Params) -> Result<()> {
    use light_sdk::compressible::{LightPreInit, LightFinalize};

    // Phase 1: Pre-init (registers compressed addresses)
    let __has_pre_init = ctx.accounts.light_pre_init(ctx.remaining_accounts, &params)?;

    // Execute original handler
    let __light_handler_result = (|| {
        ctx.accounts.user.owner = params.owner;
        Ok(())
    })();

    // Phase 2: Finalize compression on success
    if __light_handler_result.is_ok() {
        ctx.accounts.light_finalize(ctx.remaining_accounts, &params, __has_pre_init)?;
    }

    __light_handler_result
}
```

### Size Validation

Compressed accounts are validated at compile time to not exceed 800 bytes:

```rust
const _: () = {
    const COMPRESSED_SIZE: usize = 8 + <UserRecord as CompressedInitSpace>::COMPRESSED_INIT_SPACE;
    if COMPRESSED_SIZE > 800 {
        panic!("Compressed account 'UserRecord' exceeds 800-byte compressible account size limit.");
    }
};
```

### Instruction Variants

The macro supports three instruction variants based on field types:
- `PdaOnly`: Only `#[light_account(init)]` PDA fields
- `TokenOnly`: Only `#[light_account(token)]` token fields
- `Mixed`: Both PDA and token fields (most common)

Currently, only `Mixed` variant is fully implemented. `PdaOnly` and `TokenOnly` will error at runtime.

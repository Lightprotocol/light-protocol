# `#[light_program]` Attribute Macro

## 1. Overview

The `#[light_program]` attribute macro provides program-level auto-discovery and instruction generation for Light Protocol's compression system. It eliminates boilerplate by automatically discovering compressible accounts, generating variant enums and seeds structs, and wrapping instruction handlers with lifecycle hooks.

**Location**: `sdk-libs/macros/src/light_pdas/program/`

## 2. Required Macros

| Location | Macro | Purpose |
|----------|-------|---------|
| Program module | `#[light_program]` | Discovers fields, generates enums/instructions, wraps handlers |
| Accounts struct | `#[derive(Accounts, LightAccounts)]` | Both required - Anchor + Light trait impls |
| Account field | `#[light_account(init)]` | Marks PDA for compression |
| Account field | `#[light_account(init, zero_copy)]` | Marks zero-copy PDA (uses Pod serialization) |
| Account field | `#[light_account(init, token::...)]` | Creates token account with compression |
| Account field | `#[light_account(token::owner_seeds = [...])]` | Token account with PDA owner seeds |
| Account field | `#[light_account(init, mint::...)]` | Creates compressed mint |
| Account field | `#[light_account(init, associated_token::...)]` | Creates associated token account |
| State struct | `#[derive(LightAccount)]` | Generates Pack/Unpack, compression_info accessors |
| State struct | `#[derive(LightDiscriminator)]` | Generates unique 8-byte discriminator |
| State struct | `#[derive(LightHasherSha)]` | Generates SHA256 hashing via DataHasher |
| State struct | `compression_info: CompressionInfo` | Required non-Option field for compression metadata |

## 3. How It Works

### 3.1 High-Level Flow

```
+------------------+     +------------------+     +------------------+
|   User Code      | --> |   Macro at       | --> |   Generated      |
|                  |     |   Compile Time   |     |   Code           |
+------------------+     +------------------+     +------------------+
| - Program module |     | 1. Parse crate   |     | - LightAccount   |
| - Accounts       |     | 2. Find #[light_ |     |   Variant enum   |
|   structs        |     |    account] flds |     | - Seeds structs  |
| - State structs  |     | 3. Extract seeds |     | - Compress/      |
|                  |     | 4. Classify seeds|     |   Decompress ix  |
|                  |     | 5. Generate code |     | - Wrapped fns    |
+------------------+     +------------------+     +------------------+
```

### 3.2 Compile-Time Discovery

The macro reads your crate at compile time to find compressible accounts:

```
#[light_program]
#[program]
pub mod my_program {
    pub mod accounts;     <-- Macro follows this to accounts.rs
    pub mod state;        <-- And this to state.rs
    ...
}

                    |
                    v

+----------------------------------------------------------+
|                    DISCOVERY                              |
+----------------------------------------------------------+
|                                                          |
|  For each #[derive(Accounts)] struct:                    |
|                                                          |
|    1. Find #[light_account(init)] fields      --> PDAs   |
|    2. Find #[light_account(init, zero_copy)]  --> ZC PDAs|
|    3. Find #[light_account(init, token, ...)] --> Tokens |
|    4. Find #[light_account(init, mint, ...)]  --> Mints  |
|    5. Find #[light_account(init, associated_token, ...)]--> ATAs|
|    6. Find mark-only token/ata fields         --> For seeds|
|    7. Parse #[account(seeds=[...])] --> Seed expressions |
|    8. Parse #[instruction(...)]    --> Params type       |
|                                                          |
+----------------------------------------------------------+
```

### 3.3 Seed Classification and Code Generation

Seeds from `#[account(seeds = [...])]` are extracted from Anchor attributes and classified into types:

**ClassifiedSeed types** (from `sdk-libs/macros/src/light_pdas/seeds/classification.rs`):

```
+------------------------+---------------------------+----------------------------+
| Seed Expression        | Classification            | Generated Code             |
+------------------------+---------------------------+----------------------------+
| b"literal"             | Literal(Vec<u8>)          | Direct byte slice          |
| CONSTANT               | Constant { path, expr }   | crate::CONSTANT qualified  |
| authority.key()        | CtxRooted { account }     | {Type}Seeds field (Pubkey) |
| params.owner           | DataRooted { expr }       | SeedParams field (Option)  |
| max(a.key(), b.key())  | FunctionCall { ... }      | Rewritten for ctx/data     |
+------------------------+---------------------------+----------------------------+
```

**Code generation strategy:**

1. **Context seeds** (`ctx.accounts.authority`) become:
   - Fields in `{Type}Seeds` struct (unpacked Pubkey)
   - Fields in `Packed{Type}Seeds` struct (u8 index + bump)
   - Pack/Unpack trait impls for client-side serialization

2. **Data seeds** (`params.owner`) that exist on the state struct become:
   - Verification checks in the variant constructor
   - Fields in the variant enum (stored with account data)

3. **Params-only seeds** (seeds from params.* that DON'T exist on state) become:
   - Fields in `SeedParams` struct (program-wide)
   - Optional parameters for decompression

4. **Constants and literals** are used directly in PDA derivation without additional structs.

### 3.4 Generated Code Structure

```
                         GENERATED ARTIFACTS
+------------------------------------------------------------------+
|  PDA VARIANTS (per field in #[light_account(init)])             |
|  +------------------------+     +------------------------+        |
|  | UserRecordSeeds        |     | UserRecord { seeds,   |        |
|  |   pub authority: Pubkey|     |   data: UserRecord }  |        |
|  +------------------------+     +------------------------+        |
|  | PackedUserRecordSeeds  |     | PackedUserRecord {    |        |
|  |   pub authority_idx: u8|     |   seeds: PackedSeeds, |        |
|  |   pub bump: u8         |     |   data: PackedData }  |        |
|  +------------------------+     +------------------------+        |
|                                                                  |
|  TOKEN VARIANTS (per field in #[light_account(token::...)])     |
|  +------------------------+     +------------------------+        |
|  | VaultSeeds             |     | Vault(TokenDataWith   |        |
|  |   pub mint: Pubkey     |     |   Seeds<VaultSeeds>)  |        |
|  +------------------------+     +------------------------+        |
|  | PackedVaultSeeds       |     | PackedVault(TokenData |        |
|  |   pub mint_idx: u8     |     |   WithPackedSeeds<    |        |
|  |   pub bump: u8         |     |   PackedVaultSeeds>)  |        |
|  +------------------------+     +------------------------+        |
|                                                                  |
|  PROGRAM-WIDE ENUMS                                              |
|  +----------------------------------------------------------+    |
|  | LightAccountVariant                                      |    |
|  |   UserRecord { seeds: UserRecordSeeds, data: UserRecord }|    |
|  |   Vault(TokenDataWithSeeds<VaultSeeds>)                  |    |
|  +----------------------------------------------------------+    |
|  | PackedLightAccountVariant (for serialization)            |    |
|  |   UserRecord { seeds: PackedUserRecordSeeds, data: ... } |    |
|  |   Vault(TokenDataWithPackedSeeds<PackedVaultSeeds>)      |    |
|  +----------------------------------------------------------+    |
|                                                                  |
|  SEED PROVIDER TRAITS (for decompression)                        |
|  +----------------------------------------------------------+    |
|  | impl PdaSeedDerivation<UserRecordCtxSeeds, SeedParams>   |    |
|  |   for UserRecord {                                       |    |
|  |     fn derive_pda_seeds_with_accounts(...) { ... }       |    |
|  | }                                                        |    |
|  +----------------------------------------------------------+    |
|                                                                  |
|  INSTRUCTIONS                                                    |
|  +--------------------+  +--------------------+  +--------------+|
|  | decompress_        |  | compress_          |  | init/update_ ||
|  | accounts_          |  | accounts_          |  | compression_ ||
|  | idempotent         |  | idempotent         |  | config       ||
|  +--------------------+  +--------------------+  +--------------+|
|                                                                  |
|  CLIENT HELPERS                                                  |
|  +----------------------------------------------------------+    |
|  | get_user_record_seeds(authority: &Pubkey) -> (Vec, Pubkey)|   |
|  | get_vault_seeds(mint: &Pubkey) -> (Vec, Pubkey)          |    |
|  | get_vault_owner_seeds() -> (Vec, Pubkey)                 |    |
|  +----------------------------------------------------------+    |
+------------------------------------------------------------------+
```

### 3.5 Instruction Wrapping

Original instruction handlers are automatically wrapped with lifecycle hooks:

```
ORIGINAL                           WRAPPED (generated)
+---------------------------+      +----------------------------------+
| pub fn create_user(       |      | pub fn create_user(              |
|   ctx: Context<Create>,   |  ->  |   ctx: Context<Create>,          |
|   params: Params          |      |   params: Params                 |
| ) -> Result<()> {         |      | ) -> Result<()> {                |
|   ctx.accounts.user       |      |   // Phase 1: Pre-init           |
|     .owner = params.owner;|      |   let __has_pre_init = ctx       |
|   Ok(())                  |      |     .accounts.light_pre_init(    |
| }                         |      |       ctx.remaining_accounts,    |
+---------------------------+      |       &params)?;                 |
                                   |                                  |
                                   |   // Phase 2: Business logic     |
                                   |   let __user_result = {          |
                                   |     ctx.accounts.user.owner =    |
                                   |       params.owner;              |
                                   |     Ok(())                       |
                                   |   };                             |
                                   |   __user_result?;                |
                                   |                                  |
                                   |   // Phase 3: Finalize           |
                                   |   ctx.accounts.light_finalize(   |
                                   |     ctx.remaining_accounts,      |
                                   |     &params, __has_pre_init)?;   |
                                   |   Ok(())                         |
                                   | }                                |
                                   +----------------------------------+
```

**Delegation pattern**: Functions that delegate to another function (e.g., single call that moves ctx) only get pre_init wrapping, since the delegated function handles its own finalization.

### 3.6 Runtime Flows

**Create (Compression)**
```
User calls create_user
        |
        v
light_pre_init: Register address in Merkle tree
        |
        v
Business logic: Set account fields
        |
        v
light_finalize: Complete compression via CPI
        |
        v
Account exists as compressed state + temporary PDA
```

**Decompress PDAs (Read/Modify)**
```
Client fetches compressed account from indexer
        |
        v
Client calls decompress_accounts_idempotent
        |
        v
PDA recreated on-chain from compressed state
        |
        v
User interacts with standard Anchor account
```

**Decompress Token Accounts and Mints**

Token accounts (ATAs) and mints are decompressed directly via the ctoken program, not through the generated `decompress_accounts_idempotent` instruction:

```
Client fetches compressed token account/mint from indexer
        |
        v
Client calls ctoken program's decompress instruction directly
        |
        v
Token account or mint recreated on-chain
        |
        v
User interacts with decompressed ctoken account/mint
```

This separation exists because:
- **PDAs**: Program-specific, seeds defined by your program, decompressed via generated instruction
- **Token accounts/mints**: Standard ctoken format, decompressed via ctoken program

**Re-Compress (Return to compressed)**
```
Authority calls compress_accounts_idempotent
        |
        v
PDA closed, state written to Merkle tree
        |
        v
Rent returned to sponsor
```

## 4. Program Variants

The macro detects which account types are present and generates appropriate code for each variant:

| Variant | Description | Account Types Present |
|---------|-------------|----------------------|
| **PDA-only** | Only regular PDAs | `#[light_account(init)]` |
| **Token-only** | Only token accounts | `#[light_account(init, token, ...)]` or mark-only |
| **Mint-only** | Only mints | `#[light_account(init, mint, ...)]` |
| **ATA-only** | Only associated token accounts | `#[light_account(init, associated_token, ...)]` |
| **Mixed** | Multiple account types | Any combination of above |

### Variant-Specific Generation

Each variant generates only the necessary code:

**PDA-only variant**:
- `LightAccountVariant` enum with PDA types
- `{Type}Seeds` structs for PDA derivation
- `decompress_accounts_idempotent` for PDAs
- `compress_accounts_idempotent` for PDAs

**Token-only variant**:
- `TokenAccountVariant` enum
- `get_{type}_seeds()` helper functions
- Token accounts decompressed via ctoken program

**Mixed variant** (most common):
- All of the above combined
- Coordinate batching of PDA and token operations
- Single CPI context for efficiency

## 5. Generated Items Summary

| Item | Purpose | Location |
|------|---------|----------|
| `LightAccountVariant` | Unpacked enum with all compressible account types | Generated in program module |
| `PackedLightAccountVariant` | Packed enum for efficient serialization (u8 indices) | Generated in program module |
| `LightAccountData` | Wrapper struct with metadata + packed variant | Generated in program module |
| `{Type}Seeds` | Unpacked seeds struct with Pubkey fields | Generated per PDA variant |
| `Packed{Type}Seeds` | Packed seeds struct with u8 indices + bump | Generated per PDA variant |
| `{Type}CtxSeeds` | Decompression context with resolved Pubkeys | Generated per PDA variant |
| `SeedParams` | Program-wide params-only seeds struct | Generated once per program |
| `decompress_accounts_idempotent` | Recreate PDAs from compressed state | Entrypoint + processor |
| `compress_accounts_idempotent` | Compress PDAs back to Merkle tree | Entrypoint + processor + dispatch |
| `initialize_compression_config` | Setup compression config PDA | Entrypoint + accounts struct |
| `update_compression_config` | Modify compression config | Entrypoint + accounts struct |
| `get_{type}_seeds()` | Client helper for PDA derivation | Module with pub use |
| `get_{type}_owner_seeds()` | Client helper for token owner derivation | Module with pub use (token only) |

**Trait implementations:**
- `impl Pack for LightAccountVariant` - Client-side packing (cfg-gated)
- `impl Unpack for Packed{Type}Seeds` - Seed unpacking from indices
- `impl DecompressVariant for PackedLightAccountVariant` - Decompression dispatch
- `impl PdaSeedDerivation<{Type}CtxSeeds, SeedParams> for {Type}` - Seed provider for decompression
- `impl UnpackedTokenSeeds<N> for {Type}Seeds` - Token seed unpacking
- `impl PackedTokenSeeds<N> for Packed{Type}Seeds` - Token seed packing

## 6. Seed Expression Support

Seeds in `#[account(seeds = [...])]` are extracted and classified by the macro:

### Literal Seeds
```rust
seeds = [b"user", "seed"]  // Byte literals, string literals
```
→ Classified as `Literal(Vec<u8>)`, used directly in PDA derivation

### Constant Seeds
```rust
seeds = [CONSTANT, crate::AUTH_SEED.as_bytes()]
```
→ Classified as `Constant { path, expr }`, qualified to `crate::CONSTANT` or module path

### Context Account Seeds
```rust
seeds = [authority.key().as_ref(), mint.key().as_ref()]
```
→ Classified as `CtxRooted { account }`, become fields in `{Type}Seeds` struct

### Instruction Data Seeds
```rust
seeds = [params.owner.as_ref()]  // Pubkey field
seeds = [params.id.to_le_bytes().as_ref()]  // u64 with conversion
```
→ Classified as `DataRooted { root, expr }`, verified against account data during compression

### Function Call Seeds
```rust
seeds = [max_key(&authority.key(), &other.key()).as_ref()]
```
→ Classified as `FunctionCall { func_expr, args, has_as_ref }`, function qualified and args rewritten

**Seed classification** (from `sdk-libs/macros/src/light_pdas/seeds/classification.rs`):
- `classify_seed_expr()` determines seed type
- `convert_classified_to_seed_elements()` generates code for each variant
- Single-segment constants/functions are qualified with their definition module path via `CrateContext`

## 7. Zero-Copy Support

Zero-copy accounts using `AccountLoader<'info, T>` are supported with the `zero_copy` keyword:

```rust
#[account(
    init,
    payer = fee_payer,
    space = 8 + core::mem::size_of::<ZcRecord>(),
    seeds = [b"zc_record", params.owner.as_ref()],
    bump,
)]
#[light_account(init, zero_copy)]
pub zc_record: AccountLoader<'info, ZcRecord>,
```

**Zero-copy deserialization** (from `compress.rs:94-124`):
- Uses `bytemuck::from_bytes()` instead of `BorshDeserialize`
- Account data must implement `bytemuck::Pod` and `bytemuck::Zeroable`
- Discriminator (8 bytes) + Pod struct size
- Size validation uses `core::mem::size_of::<T>()`

**vs. Borsh deserialization** (default):
- Uses `AnchorDeserialize::deserialize()` for variable-length data
- Size validation uses `CompressedInitSpace` trait
- Supports String, Vec, and other dynamic types

## 8. Source Code Structure

```
sdk-libs/macros/src/light_pdas/program/
|
|-- mod.rs              # Re-exports, light_program entry point
|
|-- instructions.rs     # Main orchestration: codegen(), light_program_impl()
|                       # Discovers fields from CrateContext
|                       # Generates variant enums, seeds structs, instructions
|                       # Wraps instruction handlers with pre_init/finalize
|
|-- parsing.rs          # Core types for code generation
|                       # InstructionVariant (PdaOnly, TokenOnly, Mixed, MintOnly, AtaOnly)
|                       # TokenSeedSpec, SeedElement, InstructionDataSpec
|                       # wrap_function_with_light(), extract_context_and_params()
|                       # convert_classified_to_seed_elements()
|
|-- variant_enum.rs     # LightVariantBuilder for enum generation
|                       # Generates LightAccountVariant (unpacked)
|                       # Generates PackedLightAccountVariant (packed)
|                       # Token seed structs + Pack/Unpack impls
|                       # DecompressVariant dispatch implementation
|
|-- compress.rs         # CompressBuilder for compress instruction
|                       # generate_dispatch_fn() - discriminator-based dispatch
|                       # generate_processor() - process_compress_accounts_idempotent
|                       # generate_entrypoint() - compress_accounts_idempotent
|                       # generate_size_validation() - 800-byte limit checks
|
|-- decompress.rs       # DecompressBuilder for decompress instruction
|                       # generate_processor() - process_decompress_accounts_idempotent
|                       # generate_entrypoint() - decompress_accounts_idempotent
|                       # generate_seed_provider_impls() - PdaSeedDerivation traits
|
|-- seed_codegen.rs     # Client seed helper generation
|                       # generate_client_seed_functions() - get_{type}_seeds()
|                       # generate_ctoken_seed_provider_implementation() (deprecated)
|
|-- seed_utils.rs       # Seed derivation utilities
|                       # generate_seed_derivation_body() - find_program_address code
|                       # ctx_fields_to_set() - helper conversions
|
|-- expr_traversal.rs   # AST expression rewriting
|                       # transform_expr_for_ctx_seeds() - ctx.field -> ctx_seeds.field
|                       # Used in PDA seed derivation for decompression
|
+-- visitors.rs         # AST traversal with syn::visit
                        # FieldExtractor - extract ctx.* and data.* fields from expressions
                        # classify_seed() - seed classification entry point
                        # generate_client_seed_code() - client function parameter generation
```

**Related seed extraction** (in `sdk-libs/macros/src/light_pdas/seeds/`):
- `extract.rs` - Main extraction from Accounts structs
- `anchor_extraction.rs` - Extract seeds from #[account(seeds=[...])]
- `classification.rs` - ClassifiedSeed type determination
- `data_fields.rs` - Data field extraction and conversion detection
- `types.rs` - ExtractedSeedSpec, ExtractedTokenSpec definitions

## 9. Limitations

| Limitation | Details |
|------------|---------|
| Max size | 800 bytes per compressed account (compile-time check) |
| Module discovery | Requires `pub mod name;` pattern (not inline `mod name {}`) |
| Token authority | `#[light_account(token, ...)]` requires `token::authority = [...]` seeds |
| Zero-copy | AccountLoader requires `zero_copy` keyword; Account forbids it |

## 10. Related Documentation

- **`sdk-libs/macros/docs/accounts/architecture.md`** - `#[derive(LightAccounts)]` and trait derives
- **`sdk-libs/macros/docs/light_program/codegen.md`** - Technical code generation details
- **`sdk-libs/macros/docs/account/`** - Trait derive macros for data structs
- **`sdk-libs/sdk/`** - Runtime SDK with trait definitions

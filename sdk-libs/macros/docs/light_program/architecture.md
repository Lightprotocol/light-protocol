# `#[light_program]` Attribute Macro

## 1. Overview

The `#[light_program]` attribute macro provides program-level auto-discovery and instruction wrapping for Light Protocol's rent-free compression system. It eliminates boilerplate by automatically generating compression infrastructure from your existing Anchor code.

**Location**: `sdk-libs/macros/src/light_pdas/program/`

## 2. Required Macros

| Location | Macro | Purpose |
|----------|-------|---------|
| Program module | `#[light_program]` | Discovers fields, generates instructions, wraps handlers |
| Accounts struct | `#[derive(LightAccounts)]` | Generates `LightPreInit`/`LightFinalize` trait impls |
| Account field | `#[light_account(init)]` | Marks PDA for compression |
| Account field | `#[light_account(init, zero_copy)]` | Marks zero-copy PDA for compression |
| Account field | `#[light_account(init, token, ...)]` | Creates token account with compression |
| Account field | `#[light_account(token::authority = ...)]` | Marks existing token account (mark-only mode) |
| Account field | `#[light_account(init, mint, ...)]` | Creates compressed mint |
| Account field | `#[light_account(init, associated_token, ...)]` | Creates associated token account |
| State struct | `#[derive(LightAccount)]` | Generates unified compression traits |
| State struct | `compression_info: CompressionInfo` | Required field for compression metadata |

## 3. How It Works

### 3.1 High-Level Flow

```
+------------------+     +------------------+     +------------------+
|   User Code      | --> |   Macro at       | --> |   Generated      |
|                  |     |   Compile Time   |     |   Code           |
+------------------+     +------------------+     +------------------+
| - Program module |     | 1. Parse crate   |     | - Variant enums  |
| - Accounts       |     | 2. Find #[light_ |     | - Seeds structs  |
|   structs        |     |    account] flds |     | - Compress/      |
| - State structs  |     | 3. Extract seeds |     |   Decompress ix  |
|                  |     | 4. Generate code |     | - Wrapped fns    |
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

### 3.3 Seed Classification

Seeds from `#[account(seeds = [...])]` are classified by source:

```
+----------------------+---------------------------+------------------------+
| Seed Expression      | Classification            | Used For               |
+----------------------+---------------------------+------------------------+
| b"literal"           | Static bytes              | PDA derivation         |
| CONSTANT             | crate::CONSTANT ref       | PDA derivation         |
| authority.key()      | Context account (Pubkey)  | Variant enum field     |
| params.owner         | Instruction data field    | Seeds struct + verify  |
+----------------------+---------------------------+------------------------+
```

Context account seeds become fields in the variant enum. Instruction data seeds become fields in the Seeds struct and are verified against account data.

### 3.4 Code Generation

```
                         GENERATED ARTIFACTS
+------------------------------------------------------------------+
|                                                                  |
|  LightAccountVariant         TokenAccountVariant              |
|  +------------------------+     +------------------------+       |
|  | UserRecord { data, .. }|     | Vault { mint }         |       |
|  | PackedUserRecord {...} |     | PackedVault { mint_idx}|       |
|  | ZcRecord { ... }       |     +------------------------+       |
|  +------------------------+              |                       |
|           |                              v                       |
|           v                    get_vault_seeds()                 |
|  UserRecordSeeds               get_vault_authority_seeds()       |
|  UserRecordCtxSeeds                                              |
|                                                                  |
+------------------------------------------------------------------+
|                                                                  |
|  INSTRUCTIONS                                                    |
|  +--------------------+  +--------------------+  +--------------+|
|  | decompress_        |  | compress_          |  | init/update_ ||
|  | accounts_          |  | accounts_          |  | compression_ ||
|  | idempotent         |  | idempotent         |  | config       ||
|  +--------------------+  +--------------------+  +--------------+|
|                                                                  |
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
|   // business logic       |      |   // 1. light_pre_init           |
| }                         |      |   // 2. business logic (closure) |
+---------------------------+      |   // 3. light_finalize           |
                                   | }                                |
                                   +----------------------------------+
```

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

| Item | Purpose |
|------|---------|
| `LightAccountVariant` | Unified enum for all compressible account types (packed + unpacked) |
| `TokenAccountVariant` | Enum for token account types |
| `{Type}Seeds` | Client-side PDA derivation with seed values |
| `{Type}CtxSeeds` | Decompression context with resolved Pubkeys |
| `decompress_accounts_idempotent` | Recreate PDAs from compressed state |
| `compress_accounts_idempotent` | Compress PDAs back to Merkle tree |
| `initialize_compression_config` | Setup compression config PDA |
| `update_compression_config` | Modify compression config |
| `get_{type}_seeds()` | Client helper functions for PDA derivation |

## 6. Seed Expression Support

Seeds in `#[account(seeds = [...])]` can reference:

- **Literals**: `b"seed"` or `"seed"`
- **Constants**: `MY_SEED` (resolved as `crate::MY_SEED`)
- **Context accounts**: `authority.key().as_ref()`
- **Instruction data**: `params.owner.as_ref()` or `params.id.to_le_bytes().as_ref()`
- **Function calls**: `max_key(&a.key(), &b.key()).as_ref()`

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

Zero-copy accounts:
- Use Pod serialization instead of Borsh
- Have different decompression path
- Data types must implement `bytemuck::Pod` and `bytemuck::Zeroable`

## 8. Source Code Structure

```
sdk-libs/macros/src/light_pdas/program/
|
|-- mod.rs              # Module entry point and exports
|
|-- instructions.rs     # Main orchestration: codegen(), light_program_impl()
|                       # Generates LightAccountVariant, Seeds structs, instruction wrappers
|
|-- parsing.rs          # Core types and expression analysis
|                       # InstructionVariant enum (PdaOnly, TokenOnly, Mixed, MintOnly, AtaOnly)
|                       # TokenSeedSpec, SeedElement, InstructionDataSpec
|                       # wrap_function_with_light(), extract_context_and_params()
|
|-- visitors.rs         # Visitor-based AST traversal
|                       # FieldExtractor struct
|                       # classify_seed(), generate_client_seed_code()
|
|-- crate_context.rs    # Anchor-style crate parsing
|                       # CrateContext, ParsedModule
|                       # Module file discovery and parsing
|
|-- variant_enum.rs     # LightAccountVariant enum generation
|                       # TokenAccountVariant/PackedTokenAccountVariant generation
|                       # Pack/Unpack trait implementations
|
|-- compress.rs         # CompressAccountsIdempotent generation
|                       # CompressContext trait impl, CompressBuilder
|
|-- decompress.rs       # DecompressAccountsIdempotent generation
|                       # DecompressContext trait impl, PDA seed provider impls
|
|-- seed_codegen.rs     # Client seed function generation
|                       # TokenSeedProvider implementation generation
|
|-- seed_utils.rs       # Seed expression conversion utilities
|                       # SeedConversionConfig, seed_element_to_ref_expr()
|
+-- expr_traversal.rs   # AST expression transformation
                        # ctx.field -> ctx_seeds.field conversion
```

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

# `#[rentfree_program]` Attribute Macro

## 1. Overview

The `#[rentfree_program]` attribute macro provides program-level auto-discovery and instruction wrapping for Light Protocol's rent-free compression system. It eliminates boilerplate by automatically generating compression infrastructure from your existing Anchor code.

**Location**: `sdk-libs/macros/src/rentfree/program/`

## 2. Required Macros

| Location | Macro | Purpose |
|----------|-------|---------|
| Program module | `#[rentfree_program]` | Discovers fields, generates instructions, wraps handlers |
| Accounts struct | `#[derive(RentFree)]` | Generates `LightPreInit`/`LightFinalize` trait impls |
| Account field | `#[rentfree]` | Marks PDA for compression |
| Account field | `#[rentfree_token(authority=[...])]` | Marks token account for compression |
| State struct | `#[derive(LightCompressible)]` | Generates compression traits + `Packed{Type}` |
| State struct | `compression_info: Option<CompressionInfo>` | Required field for compression metadata |

## 3. How It Works

### 3.1 High-Level Flow

```
+------------------+     +------------------+     +------------------+
|   User Code      | --> |   Macro at       | --> |   Generated      |
|                  |     |   Compile Time   |     |   Code           |
+------------------+     +------------------+     +------------------+
| - Program module |     | 1. Parse crate   |     | - Variant enums  |
| - Accounts       |     | 2. Find #[rent-  |     | - Seeds structs  |
|   structs        |     |    free] fields  |     | - Compress/      |
| - State structs  |     | 3. Extract seeds |     |   Decompress ix  |
|                  |     | 4. Generate code |     | - Wrapped fns    |
+------------------+     +------------------+     +------------------+
```

### 3.2 Compile-Time Discovery

The macro reads your crate at compile time to find compressible accounts:

```
#[rentfree_program]
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
|    1. Find #[rentfree] fields      --> PDA accounts      |
|    2. Find #[rentfree_token] fields --> Token accounts   |
|    3. Parse #[account(seeds=[...])] --> Seed expressions |
|    4. Parse #[instruction(...)]    --> Params type       |
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
|  RentFreeAccountVariant         TokenAccountVariant              |
|  +------------------------+     +------------------------+       |
|  | UserRecord { data, .. }|     | Vault { mint }         |       |
|  | PackedUserRecord {...} |     | PackedVault { mint_idx}|       |
|  +------------------------+     +------------------------+       |
|           |                              |                       |
|           v                              v                       |
|  UserRecordSeeds               get_vault_seeds()                 |
|  UserRecordCtxSeeds            get_vault_authority_seeds()       |
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

**Decompress (Read/Modify)**
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

## 4. Generated Items Summary

| Item | Purpose |
|------|---------|
| `RentFreeAccountVariant` | Unified enum for all compressible account types (packed + unpacked) |
| `TokenAccountVariant` | Enum for token account types |
| `{Type}Seeds` | Client-side PDA derivation with seed values |
| `{Type}CtxSeeds` | Decompression context with resolved Pubkeys |
| `decompress_accounts_idempotent` | Recreate PDAs from compressed state |
| `compress_accounts_idempotent` | Compress PDAs back to Merkle tree |
| `initialize_compression_config` | Setup compression config PDA |
| `update_compression_config` | Modify compression config |
| `get_{type}_seeds()` | Client helper functions for PDA derivation |
| `RentFreeInstructionError` | Error codes for compression operations |

## 5. Seed Expression Support

Seeds in `#[account(seeds = [...])]` can reference:

- **Literals**: `b"seed"` or `"seed"`
- **Constants**: `MY_SEED` (resolved as `crate::MY_SEED`)
- **Context accounts**: `authority.key().as_ref()`
- **Instruction data**: `params.owner.as_ref()` or `params.id.to_le_bytes().as_ref()`
- **Function calls**: `max_key(&a.key(), &b.key()).as_ref()`

## 6. Limitations

| Limitation | Details |
|------------|---------|
| Max size | 800 bytes per compressed account (compile-time check) |
| Module discovery | Requires `pub mod name;` pattern (not inline `mod name {}`) |
| Instruction variants | Only `Mixed` (PDA + token) fully implemented |
| Token authority | `#[rentfree_token]` requires `authority = [...]` seeds |

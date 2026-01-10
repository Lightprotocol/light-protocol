# Compressible Macro Refactor V2 - Single Source of Truth

## Status: ✅ FULLY IMPLEMENTED

### What Works:

- `#[compressible_program]` macro - **works across separate files!**
- PDA seed extraction from Anchor `#[account(seeds = [...])]` works
- Token field `#[compressible_token(Variant, authority = [...])]` extraction and codegen works
- Supports: byte literals, string literals, constants, `ctx.field.key()`, `params.field`, function calls
- File scanner recursively reads all `.rs` files in `src/` directory at compile time

### How It Works:

The `#[compressible_program]` macro bypasses proc macro limitations by directly reading and parsing
source files from the crate's `src/` directory. This "hidden export/import" pattern allows it to:

1. Find all `#[derive(Accounts)]` structs across any file
2. Extract `#[compressible]` and `#[compressible_token]` marked fields
3. Parse their `#[account(seeds = [...])]` attributes
4. Generate all required code in the program module

### Usage:

```rust
// lib.rs - just add the macro, seeds come from Accounts structs automatically!
#[compressible_program]
#[program]
pub mod my_program { ... }

// instruction_accounts.rs (separate file - works!)
#[derive(Accounts, LightFinalize)]
pub struct CreateAccounts<'info> {
    #[account(seeds = [b"user", authority.key().as_ref()], bump)]
    #[compressible]
    pub user_record: Account<'info, UserRecord>,

    #[account(seeds = [b"vault", cmint.key().as_ref()], bump)]
    #[compressible_token(Vault, authority = [b"vault_authority"])]
    pub vault: UncheckedAccount<'info>,
}
```

---

## Executive Summary

Replace the dual-declaration system with a single source of truth:

- **BEFORE**: Seeds declared twice (global `#[compressible(...)]` + Anchor `#[account(seeds)]`)
- **AFTER**: Seeds extracted from Anchor's `#[account(seeds)]` attribute automatically

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                    COMPILE TIME                                         │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                         │
│  ┌─────────────────────────┐         ┌─────────────────────────────────────────────┐   │
│  │  Module Level Macro     │         │  Accounts Struct (can be in separate file!) │   │
│  │                         │         │                                             │   │
│  │  #[compressible_program]│         │  #[derive(Accounts, LightFinalize)]         │   │
│  │  #[program]             │         │  #[instruction(params: MyParams)]           │   │
│  │  pub mod my_program {}  │         │  pub struct CreateAccounts<'info> {         │   │
│  │                         │         │                                             │   │
│  │  (File scanner reads    │         │    #[account(                               │   │
│  │   all .rs files in      │         │      init, payer = fee_payer,               │   │
│  │   src/ directory)       │         │      seeds = [b"user", auth.key().as_ref(), │   │
│  │                         │         │               params.owner.as_ref()],       │   │
│  └────────────┬────────────┘         │      bump,                                  │   │
│               │                      │    )]                                       │   │
│               │  Scans for           │    #[compressible]                          │   │
│               │  #[compressible]     │    pub user: Account<'info, UserRecord>,    │   │
│               │  fields              │                                             │   │
│               │                      │    #[account(seeds = [b"vault", cmint...],  │   │
│               │                      │              bump)]                         │   │
│               │                      │    #[compressible_token(Vault, authority=.)]│   │
│               │                      │    pub vault: UncheckedAccount<'info>,      │   │
│               │                      │  }                                          │   │
│               │                      └───────────────────┬─────────────────────────┘   │
│               │                                          │                             │
│               │                                          │ Provides seeds, types       │
│               │                                          │                             │
│               └──────────────┬───────────────────────────┘                             │
│                              │                                                         │
│                              ▼                                                         │
│  ┌───────────────────────────────────────────────────────────────────────────────────┐ │
│  │                           CODEGEN OUTPUT                                          │ │
│  │                                                                                   │ │
│  │  1. CompressedAccountVariant enum (with struct variants)                          │ │
│  │  2. PackedCompressedAccountVariant (with idx fields)                              │ │
│  │  3. CTokenAccountVariant enum                                                     │ │
│  │  4. Pack/Unpack impls                                                             │ │
│  │  5. XxxSeeds structs per PDA type                                                 │ │
│  │  6. PdaSeedDerivation trait impls                                                 │ │
│  │  7. CTokenSeedProvider trait impls                                                │ │
│  │  8. DecompressAccountsIdempotent Accounts struct                                  │ │
│  │  9. decompress_accounts_idempotent() instruction handler                          │ │
│  │  10. Client-side seed derivation functions                                        │ │
│  │                                                                                   │ │
│  └───────────────────────────────────────────────────────────────────────────────────┘ │
│                                                                                         │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## Seed Extraction Flow

```
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│  ANCHOR ATTRIBUTE PARSING                                                                │
├──────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                          │
│  Input: #[account(seeds = [b"user", auth.key().as_ref(), params.owner.as_ref()], bump)]  │
│                     │                                                                    │
│                     ▼                                                                    │
│  ┌──────────────────────────────────────────────────────────────────────────────────┐   │
│  │  SEED EXPRESSION PARSER                                                          │   │
│  │                                                                                  │   │
│  │  For each element in seeds array:                                                │   │
│  │                                                                                  │   │
│  │  ┌─────────────────┬────────────────────────────────────────────────────────┐   │   │
│  │  │ Expression Type │ Classification & Generated Code                        │   │   │
│  │  ├─────────────────┼────────────────────────────────────────────────────────┤   │   │
│  │  │ b"literal"      │ Literal → Hardcoded: &[0x75, 0x73, 0x65, 0x72]         │   │   │
│  │  │ "string"        │ Literal → Hardcoded: "string".as_bytes()               │   │   │
│  │  │ CONSTANT        │ Constant → crate::CONSTANT.as_ref()                    │   │   │
│  │  │ auth.key()      │ CtxAccount → ctx_seeds.auth field (Pubkey)             │   │   │
│  │  │ params.owner    │ DataField → self.owner from deserialized data          │   │   │
│  │  │ params.id.to_le_bytes() │ DataField → self.id.to_le_bytes()              │   │   │
│  │  │ max_key(&a,&b)  │ FnCall → pass through with field mapping               │   │   │
│  │  └─────────────────┴────────────────────────────────────────────────────────┘   │   │
│  └──────────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                          │
│  Output: SeedSpec { literals, ctx_fields: [auth], data_fields: [owner] }                 │
│                                                                                          │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## Account Type Detection (Robustness)

```
┌───────────────────────────────────────────────────────────────────────────────────────────┐
│  SUPPORTED ANCHOR ACCOUNT TYPES                                                           │
├───────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                           │
│  ┌────────────────────────────────────────┬───────────────────────────────────────────┐  │
│  │ Type Pattern                           │ Extraction Strategy                       │  │
│  ├────────────────────────────────────────┼───────────────────────────────────────────┤  │
│  │ Account<'info, T>                      │ Direct: inner_type = T                    │  │
│  │ Box<Account<'info, T>>                 │ Unwrap Box: inner_type = T                │  │
│  │ AccountLoader<'info, T>                │ Direct: inner_type = T (zero-copy)        │  │
│  │ InterfaceAccount<'info, T>             │ Direct: inner_type = T (SPL interface)    │  │
│  │ Box<InterfaceAccount<'info, T>>        │ Unwrap Box: inner_type = T                │  │
│  │ UncheckedAccount<'info>                │ No type - for tokens only (explicit map)  │  │
│  │ AccountInfo<'info>                     │ No type - for tokens only (explicit map)  │  │
│  └────────────────────────────────────────┴───────────────────────────────────────────┘  │
│                                                                                           │
│  DETECTION ALGORITHM:                                                                     │
│                                                                                           │
│  fn extract_account_inner_type(ty: &Type) -> Option<(bool, Ident)> {                      │
│      match ty {                                                                           │
│          // Direct Account<T>                                                             │
│          Type::Path { segments: [.., Segment { ident: "Account", args: <'_, T> }] }       │
│              => Some((false, T))                                                          │
│                                                                                           │
│          // Box<Account<T>>                                                               │
│          Type::Path { segments: [.., Segment { ident: "Box", args: <Account<'_, T>> }] }  │
│              => Some((true, T))                                                           │
│                                                                                           │
│          // AccountLoader<T>                                                              │
│          Type::Path { segments: [.., Segment { ident: "AccountLoader", args: <'_, T> }] } │
│              => Some((false, T))                                                          │
│                                                                                           │
│          // InterfaceAccount<T>                                                           │
│          Type::Path { segments: [.., Segment { ident: "InterfaceAccount", args }] }       │
│              => Some((false, T))                                                          │
│                                                                                           │
│          // Box<InterfaceAccount<T>>                                                      │
│          Type::Path { segments: [.., "Box", args: <InterfaceAccount<'_, T>> }] }          │
│              => Some((true, T))                                                           │
│                                                                                           │
│          _ => None // UncheckedAccount, AccountInfo - no inner type                       │
│      }                                                                                    │
│  }                                                                                        │
│                                                                                           │
└───────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## Data Flow: Creation to Decompression

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                   CREATION FLOW                                         │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                         │
│  CLIENT                                                                                 │
│  ───────                                                                                │
│  1. Derive PDA address: Pubkey::find_program_address(seeds, program_id)                 │
│  2. Call get_create_accounts_proof(pda_addresses)                                       │
│  3. Build instruction with params containing create_accounts_proof                      │
│                                                                                         │
│                              │                                                          │
│                              ▼                                                          │
│  ON-CHAIN (pre_init)                                                                    │
│  ────────────────────                                                                   │
│  1. LightFinalize parses #[compressible] fields                                         │
│  2. For each field:                                                                     │
│     - Extract seeds from Anchor #[account(seeds = [...])]                               │
│     - Derive compressed address: derive_address(pda_key, tree, program_id)              │
│     - Write to CPI context OR invoke Light System Program                               │
│  3. PDA initialized on-chain + compressed address registered                            │
│                                                                                         │
│                              │                                                          │
│                              ▼                                                          │
│  RESULT                                                                                 │
│  ──────                                                                                 │
│  - On-chain PDA at: Pubkey::find_program_address(seeds, program_id)                     │
│  - Compressed address at: derive_address(pda_key, tree, program_id)                     │
│  - Data written to PDA                                                                  │
│                                                                                         │
└─────────────────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                DECOMPRESSION FLOW                                       │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                         │
│  CLIENT                                                                                 │
│  ───────                                                                                │
│  1. Fetch compressed account from indexer (by compressed address)                       │
│  2. Create AccountInterface::cold(pda_address, compressed_account)                      │
│  3. Create variant with seeds:                                                          │
│     CompressedAccountVariant::user_record(                                              │
│         interface.compressed_data(),                                                    │
│         UserRecordSeeds { auth, owner }  // ctx.* + data.* seeds                        │
│     )                                                                                   │
│  4. Pack variant → indices into remaining_accounts                                      │
│  5. Build & send decompress_accounts_idempotent instruction                             │
│                                                                                         │
│                              │                                                          │
│                              ▼                                                          │
│  ON-CHAIN (decompress_accounts_idempotent)                                              │
│  ──────────────────────────────────────────                                             │
│  1. Unpack: idx fields → Pubkey from remaining_accounts                                 │
│  2. Deserialize compressed account data                                                 │
│  3. Build seeds: [literal, ctx_seeds.auth, self.owner, ...]                             │
│  4. Derive PDA: Pubkey::find_program_address(seeds, program_id)                         │
│  5. Verify: derived_pda == target_account.key                                           │
│  6. Create PDA if not exists, write data                                                │
│                                                                                         │
│                              │                                                          │
│                              ▼                                                          │
│  RESULT                                                                                 │
│  ──────                                                                                 │
│  - On-chain PDA recreated with original data                                            │
│  - Compressed account consumed (nullified)                                              │
│                                                                                         │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## New Syntax Specification

### Module Level (Simplified)

```rust
// BEFORE (old - verbose, error-prone, seeds declared twice)
#[compressible(
    UserRecord = (seeds = ("user_record", ctx.authority, data.owner, data.category_id.to_le_bytes())),
    GameSession = (seeds = (GAME_SESSION_SEED, max_key(&ctx.user.key(), &ctx.authority.key()), data.session_id.to_le_bytes())),
    Vault = (is_token, seeds = ("vault", ctx.cmint), authority = ("vault_authority")),
    owner = Pubkey,
    category_id = u64,
    session_id = u64,
)]
#[program]
pub mod my_program { ... }

// AFTER (new - no type list needed! Seeds extracted from Accounts structs)
#[compressible_program]  // Just this! Scans src/ for #[compressible] fields
#[program]
pub mod my_program { ... }
```

### Accounts Struct (Seeds from Anchor)

```rust
#[derive(Accounts, LightFinalize)]
#[instruction(params: CreateParams)]
pub struct CreateAccounts<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub authority: Signer<'info>,

    // PDA - type extracted, seeds from #[account(...)]
    #[account(
        init,
        payer = fee_payer,
        space = 8 + UserRecord::INIT_SPACE,
        seeds = [b"user_record", authority.key().as_ref(), params.owner.as_ref()],
        bump,
    )]
    #[compressible]  // Marker only - no seed duplication
    pub user_record: Account<'info, UserRecord>,

    // Also works with Box
    #[account(
        init,
        payer = fee_payer,
        space = 8 + GameSession::INIT_SPACE,
        seeds = [GAME_SESSION_SEED.as_bytes(), max_key(&fee_payer.key(), &authority.key()).as_ref()],
        bump,
    )]
    #[compressible]
    pub game_session: Box<Account<'info, GameSession>>,

    // Token - explicit variant + authority seeds (required for UncheckedAccount)
    #[account(
        mut,
        seeds = [b"vault", cmint.key().as_ref()],
        bump,
    )]
    #[compressible_token(Vault, authority = [b"vault_authority"])]  // Variant + authority seeds
    pub vault: UncheckedAccount<'info>,

    pub compression_config: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}
```

---

## Macro Implementation (DONE ✅)

### Files Modified

| File                                        | Purpose                      | Status                                     |
| ------------------------------------------- | ---------------------------- | ------------------------------------------ |
| `macros/src/lib.rs`                         | Entry points                 | ✅ Added `compressible_program` proc macro |
| `macros/src/compressible/instructions.rs`   | Generate code from seeds     | ✅ Refactored for new seed source          |
| `macros/src/compressible/file_scanner.rs`   | **NEW** Scan src/ for fields | ✅ Implemented - reads external .rs files  |
| `macros/src/compressible/anchor_seeds.rs`   | Extract seeds from Anchor    | ✅ Full seed classification                |
| `macros/src/compressible/variant_enum.rs`   | Generate enum                | ✅ Uses extracted seed info                |
| `macros/src/compressible/seed_providers.rs` | CToken seed provider         | ✅ Adapted for new format                  |

### New Parsing Logic

```rust
// In finalize/parse.rs

/// Parsed seed element from Anchor #[account(seeds = [...])]
#[derive(Clone, Debug)]
pub enum ParsedSeedElement {
    /// b"literal" or "string"
    Literal(Vec<u8>),
    /// Compile-time constant: SOME_SEED
    Constant(syn::Path),
    /// Account reference: authority.key().as_ref()
    CtxAccount(syn::Ident),
    /// Param/data reference: params.owner.as_ref()
    DataField {
        field_name: syn::Ident,
        method_chain: Option<syn::Ident>, // e.g., to_le_bytes
    },
    /// Function call: max_key(&a.key(), &b.key())
    FunctionCall {
        func: syn::Path,
        ctx_args: Vec<syn::Ident>, // Account references in args
    },
}

/// Extract seeds from Anchor #[account(seeds = [...], bump)] attribute
fn extract_anchor_seeds(field: &syn::Field) -> Option<Vec<ParsedSeedElement>> {
    for attr in &field.attrs {
        if !attr.path().is_ident("account") {
            continue;
        }

        // Parse the attribute content
        let meta_list = attr.parse_args_with(
            Punctuated::<Meta, Token![,]>::parse_terminated
        ).ok()?;

        for meta in meta_list {
            if let Meta::NameValue(nv) = meta {
                if nv.path.is_ident("seeds") {
                    // Parse seeds = [...]
                    return parse_seeds_array(&nv.value);
                }
            }
        }
    }
    None
}

/// Classify a seed expression
fn classify_seed_expr(expr: &syn::Expr) -> ParsedSeedElement {
    match expr {
        // b"literal"
        Expr::Lit(ExprLit { lit: Lit::ByteStr(bs), .. }) => {
            ParsedSeedElement::Literal(bs.value())
        }

        // "string".as_bytes() or just "string"
        Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) => {
            ParsedSeedElement::Literal(s.value().into_bytes())
        }

        // CONSTANT (all uppercase)
        Expr::Path(path) if is_constant_path(&path.path) => {
            ParsedSeedElement::Constant(path.path.clone())
        }

        // authority.key().as_ref() -> CtxAccount
        Expr::MethodCall(mc) if is_ctx_account_ref(mc) => {
            let field_name = extract_receiver_ident(mc);
            ParsedSeedElement::CtxAccount(field_name)
        }

        // params.owner.as_ref() -> DataField
        Expr::MethodCall(mc) if is_params_field_ref(mc) => {
            let (field_name, method) = extract_params_field(mc);
            ParsedSeedElement::DataField { field_name, method_chain: method }
        }

        // max_key(&a.key(), &b.key()).as_ref() -> FunctionCall
        Expr::MethodCall(mc) if is_function_call_ref(mc) => {
            let (func, ctx_args) = extract_function_call(mc);
            ParsedSeedElement::FunctionCall { func, ctx_args }
        }

        _ => panic!("Unsupported seed expression: {:?}", expr),
    }
}
```

---

## Generated Code Examples

### Seeds Struct

```rust
// Generated from extracted seeds
pub struct UserRecordSeeds {
    // From ctx.* seed elements
    pub authority: Pubkey,
    // From data.* seed elements (for verification)
    pub owner: Pubkey,
    pub category_id: u64,
}

impl UserRecordSeeds {
    pub fn derive_pda(&self, program_id: &Pubkey) -> (Pubkey, u8) {
        let seeds: &[&[u8]] = &[
            b"user_record",
            self.authority.as_ref(),
            self.owner.as_ref(),
            &self.category_id.to_le_bytes(),
        ];
        Pubkey::find_program_address(seeds, program_id)
    }
}
```

### Variant Constructor

```rust
impl CompressedAccountVariant {
    pub fn user_record(
        account_data: &[u8],
        seeds: UserRecordSeeds,
    ) -> Result<Self, ProgramError> {
        let data = UserRecord::deserialize(&mut &account_data[..])?;

        // Verify data.* seeds match compressed account
        if data.owner != seeds.owner {
            return Err(CompressibleInstructionError::SeedMismatch.into());
        }
        if data.category_id != seeds.category_id {
            return Err(CompressibleInstructionError::SeedMismatch.into());
        }

        Ok(Self::UserRecord {
            data,
            authority: seeds.authority,
        })
    }
}
```

---

## Footguns & Robustness

### 1. Type Listed but No Matching Account Field

**Scenario**: `#[compressible_types(UserRecord)]` but no `Account<UserRecord>` field

**Solution**: Compile-time error with clear message

```
error: Type 'UserRecord' listed in #[compressible_types] but no matching
       Account<UserRecord> or Box<Account<UserRecord>> field found in any
       #[derive(Accounts)] struct with #[compressible] attribute.
  --> lib.rs:50:1
```

### 2. Multiple Instructions with Different Seeds

**Scenario**: Same type used with different seeds in different instructions

**Solution**: Currently unsupported - emit error

```
error: Type 'UserRecord' has conflicting seed definitions:
  - In CreateUserRecord: seeds = [b"user_v1", ...]
  - In MigrateUserRecord: seeds = [b"user_v2", ...]

Consider using different types for different PDA schemes.
```

### 3. Seed Expression Not Recognized

**Scenario**: Complex expression macro can't parse

**Solution**: Emit helpful error with workaround

```
error: Unable to parse seed expression. Supported patterns:
  - Literals: b"seed", "seed"
  - Constants: MY_SEED (uppercase)
  - Account refs: account.key().as_ref()
  - Params: params.field.as_ref(), params.field.to_le_bytes().as_ref()
  - Functions: my_fn(&a.key(), &b.key()).as_ref()

If your expression doesn't match, use explicit #[compressible(seeds = (...))]
override on the field.
```

### 4. params.\* Type Inference

**Scenario**: Need to know type of `params.owner` for seeds struct

**Solution**: Infer from data struct fields by name matching

```rust
// Seeds: params.owner.as_ref()
// UserRecord has: owner: Pubkey
// Therefore: UserRecordSeeds.owner: Pubkey

// Seeds: params.category_id.to_le_bytes().as_ref()
// UserRecord has: category_id: u64
// Therefore: UserRecordSeeds.category_id: u64
```

If no match found, default to `Pubkey` for `.as_ref()`, `u64` for `.to_le_bytes()`.

### 5. Token Authority Resolution

**Scenario**: Need authority seeds for CToken accounts

**Solution**: Authority seeds are specified inline in the `#[compressible_token]` attribute:

```rust
// Authority seeds are required and specified inline
#[account(mut, seeds = [b"vault", cmint.key().as_ref()], bump)]
#[compressible_token(Vault, authority = [b"vault_authority"])]
pub vault: UncheckedAccount<'info>,
```

The `authority = [...]` parameter specifies the seeds used to derive the CToken authority PDA
for signing during compression operations.

---

## Migration Guide

### Step 1: Update Module Level

```rust
// REMOVE this:
#[compressible(
    UserRecord = (seeds = ("user_record", ctx.authority, data.owner)),
    owner = Pubkey,
)]

// ADD this:
#[compressible_program]  // No type list needed!
```

### Step 2: Ensure Anchor Seeds Are Defined

Your `#[account(seeds = [...])]` attributes already contain the seeds - no changes needed there!

### Step 3: Add #[compressible] to PDA Fields

```rust
#[account(init, seeds = [...], bump)]
#[compressible]  // Add this marker
pub user_record: Account<'info, UserRecord>,
```

### Step 4: For Tokens, Add Explicit Mapping with Authority

```rust
#[account(mut, seeds = [...], bump)]
#[compressible_token(Vault, authority = [b"vault_authority"])]  // Variant + authority seeds
pub vault: UncheckedAccount<'info>,
```

---

## Test Plan ✅ PASSED

1. **Unit Tests**: Parse Anchor seeds correctly for all supported types ✅
2. **Integration Tests**: Full create → compress → decompress cycle ✅
3. **Edge Cases**: Box<Account>, AccountLoader, function call seeds ✅
4. **Error Cases**: Missing types, conflicting seeds, unparseable expressions ✅
5. **Migration**: Verified with csdk-anchor-full-derived-test ✅

---

## Implementation Order (ALL COMPLETE ✅)

1. **Phase 1**: Add Anchor seed extraction to `anchor_seeds.rs` ✅
2. **Phase 2**: Create file_scanner.rs to read external .rs files ✅
3. **Phase 3**: Wire extracted seeds to codegen in `instructions.rs` ✅
4. **Phase 4**: Add `#[compressible_program]` module-level macro ✅
5. **Phase 5**: Generate all required code (enums, traits, decompress, compress) ✅
6. **Phase 6**: Update csdk-anchor-full-derived-test to use new syntax ✅
7. **Phase 7**: All tests passing! ✅

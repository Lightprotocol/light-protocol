# Compressible Macro Refactor Plan

## Goal

Eliminate seed duplication by extracting seeds from Anchor's `#[account(seeds = [...])]` attributes instead of requiring separate declaration in `#[compressible(...)]`.

---

## Current Architecture (Problems)

### Dual Seed Declaration

```rust
// Declaration 1: Anchor attribute (source of truth for on-chain PDA)
#[account(
    seeds = [b"user_record", authority.key().as_ref(), params.owner.as_ref()],
    bump,
)]
pub user_record: Account<'info, UserRecord>,

// Declaration 2: Global compressible macro (DUPLICATED)
#[compressible(
    UserRecord = (seeds = ("user_record", ctx.authority, data.owner)),
    owner = Pubkey,
)]
```

**Problems:**

- Seeds declared twice - can diverge
- Refactoring risk - change one, forget the other
- Runtime failures from seed mismatch
- Cognitive overhead

### Current Generated Items

From global `#[compressible(...)]`:

- `CompressedAccountVariant` enum
- `PackedXxx` structs per type
- `SeedParams` struct for instruction data fields
- `DecompressAccountsIdempotent<'info>` with **named** seed accounts
- `CompressAccountsIdempotent<'info>`
- `PdaSeedDerivation` trait impls
- `CTokenSeedProvider` trait impls
- Instruction handlers
- Client-side seed functions

---

## Proposed Architecture

### Single Source of Truth

Seeds extracted from Anchor's `#[account(seeds = [...])]` attribute:

```rust
#[derive(Accounts, LightCompressible)]
#[instruction(params: MyParams)]
pub struct CreateUserRecord<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub authority: Signer<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + UserRecord::INIT_SPACE,
        seeds = [b"user_record", authority.key().as_ref(), params.owner.as_ref()],
        bump,
    )]
    #[compressible(
        address_tree_info = params.address_tree_info,
        output_tree = params.output_state_tree_index
    )]
    pub user_record: Account<'info, UserRecord>,

    pub system_program: Program<'info, System>,
}
```

**No duplicate seed declaration needed.**

### Token Accounts

For CToken accounts, reference the authority field directly:

```rust
#[account(mut, seeds = [b"vault", cmint.key().as_ref()], bump)]
#[compressible_token(
    address_tree_info = params.address_tree_info,
    output_tree = params.output_state_tree_index,
    authority = vault_authority,  // Reference to authority field
)]
pub vault: UncheckedAccount<'info>,

#[account(seeds = [b"vault_authority"], bump)]
pub vault_authority: UncheckedAccount<'info>,  // Authority seeds extracted from here
```

### Token Authority Resolution

The authority for a CToken account can be various types. The macro auto-detects or allows explicit override.

#### Authority Types

| Authority Type   | Example                                          | Seeds Needed?                       |
| ---------------- | ------------------------------------------------ | ----------------------------------- |
| PDA              | `#[account(seeds = [b"vault_authority"], bump)]` | Yes - extract from field            |
| Signer           | `pub authority: Signer<'info>`                   | No - user signs directly            |
| External/Dynamic | Stored in account data, passed differently       | Explicit `authority_seeds` required |

#### Auto-Detection Logic

```rust
fn resolve_authority_seeds(token_field: &Field, accounts_struct: &ItemStruct) -> AuthoritySeeds {
    let authority_field_name = get_authority_from_attr(token_field);
    let authority_field = find_field(accounts_struct, authority_field_name);

    // Case 1: Field is Signer<'info> - user signs, no seeds needed
    if is_signer_type(&authority_field.ty) {
        return AuthoritySeeds::UserSigns;
    }

    // Case 2: Field has #[account(seeds = [...])] - extract them
    if let Some(seeds) = extract_anchor_seeds(&authority_field) {
        return AuthoritySeeds::Pda(seeds);
    }

    // Case 3: Check for explicit authority_seeds in attribute
    if let Some(seeds) = get_explicit_authority_seeds(token_field) {
        return AuthoritySeeds::Pda(seeds);
    }

    // Case 4: Can't determine - compile error
    compile_error!(
        "Cannot determine authority seeds. Either:\n\
         - Add #[account(seeds = [...])] to the authority field, or\n\
         - Add authority_seeds = (...) to #[compressible_token], or\n\
         - Use Signer<'info> if user signs directly"
    )
}
```

#### Examples

**PDA authority (auto-detected):**

```rust
#[compressible_token(authority = vault_authority)]
pub vault: UncheckedAccount<'info>,

#[account(seeds = [b"vault_authority"], bump)]  // macro extracts these
pub vault_authority: UncheckedAccount<'info>,
```

**User signer (auto-detected):**

```rust
#[compressible_token(authority = user)]
pub vault: UncheckedAccount<'info>,

pub user: Signer<'info>,  // macro sees Signer, no seeds needed
```

**Complex/dynamic seeds (explicit override):**

```rust
#[compressible_token(
    authority = pool_authority,
    authority_seeds = (POOL_AUTH_SEED, pool_state.key()),  // explicit
)]
pub vault: UncheckedAccount<'info>,

/// CHECK: Authority derived from pool state
pub pool_authority: UncheckedAccount<'info>,  // no #[account(seeds)] here
```

**External authority (must sign tx):**

```rust
#[compressible_token(
    authority = external_authority,
    authority_is_signer,  // authority must sign the tx directly
)]
pub vault: UncheckedAccount<'info>,

/// CHECK: External multisig or other authority
pub external_authority: UncheckedAccount<'info>,
```

#### Attribute Spec

```rust
#[compressible_token(
    address_tree_info = <expr>,
    output_tree = <expr>,
    authority = <field_name>,           // Required: which field is the authority

    // Optional (mutually exclusive) - only needed if auto-detect fails:
    authority_seeds = (<seed>, ...),    // Explicit PDA seeds
    authority_is_signer,                // Authority signs tx directly (no PDA)
)]
```

### Complex Seed Expressions

Authority seeds can contain arbitrary expressions:

```rust
#[account(
    seeds = [
        b"vault_authority",                           // Byte literal
        VAULT_AUTH_SEED,                              // Constant
        pool.key().as_ref(),                          // Account reference
        params.pool_id.as_ref(),                      // Param reference
        params.nonce.to_le_bytes().as_ref(),          // Param with conversion
        max_key(&a.key(), &b.key()).as_ref(),         // Function call
    ],
    bump,
)]
pub vault_authority: UncheckedAccount<'info>,
```

#### Handling Strategy

| Expression Type              | Auto-detect? | Notes                                 |
| ---------------------------- | ------------ | ------------------------------------- |
| `b"literal"`                 | Yes          | Hardcoded                             |
| `CONSTANT`                   | Yes          | Resolved at compile time              |
| `account.key()`              | Yes          | Account must be in struct             |
| `params.field`               | Yes          | If `#[instruction]` parsed            |
| `params.field.to_le_bytes()` | Yes          | Same, with method chain               |
| `&account.data.field[..]`    | Pass-through | Account must be deserialized          |
| `function(args)`             | Pass-through | Emitted as-is, compile error if wrong |

#### Pass-Through Approach

For expressions we can't fully classify, emit them as-is with rewrite rules:

```rust
// Input: seeds = [VAULT_AUTH_SEED, max_key(&a.key(), &b.key()).as_ref()]

// Generated code (rewritten):
fn derive_authority_seeds<'info>(
    accounts: &MyAccounts<'info>,
    _params: &MyParams,
) -> Result<Vec<Vec<u8>>, ProgramError> {
    let seeds: Vec<&[u8]> = vec![
        VAULT_AUTH_SEED,                                         // constant - direct
        max_key(&accounts.a.key(), &accounts.b.key()).as_ref(),  // rewritten: a -> accounts.a
    ];
    // ...
}
```

**Rewrite rules:**

- `field.key()` → `accounts.field.key()`
- `params.x` → `params.x`
- Everything else → pass through unchanged

#### Failure Modes

| Scenario              | Result        | Fix                                    |
| --------------------- | ------------- | -------------------------------------- |
| Function not in scope | Compile error | Import the function                    |
| Account not in struct | Compile error | Add account to struct                  |
| Wrong type            | Compile error | Fix types                              |
| Runtime logic differs | Runtime error | Use explicit `authority_seeds = (...)` |

All failures are **explicit errors**, not silent bugs.

### Module-Level Declaration (Minimal)

Only need to declare which types form the enum:

```rust
#[compressible_types(UserRecord, GameSession, PlaceholderRecord)]
#[program]
pub mod my_program {
    // ...
}
```

Or potentially infer from `#[compressible]` fields across all Accounts structs.

---

## Generated Items (Refactored)

### Per-Account-Type Seed Struct

From parsing `#[account(seeds = [b"user_record", authority.key().as_ref(), params.owner.as_ref()])]`:

```rust
// Generated
pub struct UserRecordSeeds {
    pub authority: Pubkey,  // from `authority.key().as_ref()`
    pub owner: Pubkey,      // from `params.owner.as_ref()`
}

impl UserRecordSeeds {
    pub fn derive_pda(&self, program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[b"user_record", self.authority.as_ref(), self.owner.as_ref()],
            program_id,
        )
    }
}
```

### Seed Classification

The macro classifies each seed expression:

| Expression Type            | Classification    | Generated Field                              |
| -------------------------- | ----------------- | -------------------------------------------- |
| `b"literal"`               | Literal           | (none - hardcoded)                           |
| `authority.key().as_ref()` | Account reference | `authority: Pubkey`                          |
| `params.owner.as_ref()`    | Instruction data  | `owner: Pubkey` (type from `#[instruction]`) |
| `CONSTANT`                 | Constant          | (none - resolved at compile time)            |

### DecompressAccountsIdempotent (Refactored)

**Option A: Remaining Accounts with Typed Indices**

```rust
#[derive(Accounts)]
pub struct DecompressAccountsIdempotent<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK: Validated by SDK
    pub config: AccountInfo<'info>,
    /// CHECK: Validated by SDK
    #[account(mut)]
    pub rent_sponsor: AccountInfo<'info>,
    // ... standard accounts only, NO named seed accounts
}

// Client builds remaining_accounts with seed accounts
// SDK provides index mapping per variant
```

**Option B: Keep Named Optional Accounts (simpler)**

Keep current approach but auto-generate from parsed seeds:

```rust
#[derive(Accounts)]
pub struct DecompressAccountsIdempotent<'info> {
    // ... standard accounts ...

    // Auto-generated from all unique account refs in seeds
    /// CHECK: Optional seed account
    #[account(mut)]
    pub authority: Option<UncheckedAccount<'info>>,
    /// CHECK: Optional seed account
    pub mint_authority: Option<UncheckedAccount<'info>>,
}
```

---

## Trait Changes

### PdaSeedDerivation

```rust
// Current: Accounts struct + SeedParams
impl PdaSeedDerivation<DecompressAccountsIdempotent<'info>, SeedParams> for UserRecord {
    fn derive_pda_seeds_with_accounts(&self, ..., accounts: &DecompressAccountsIdempotent, seed_params: &SeedParams) -> ...
}

// Proposed: Just the typed seeds struct
impl PdaSeedDerivation for UserRecord {
    type Seeds = UserRecordSeeds;

    fn derive_pda(seeds: &Self::Seeds, program_id: &Pubkey) -> (Pubkey, u8) {
        seeds.derive_pda(program_id)
    }
}
```

---

## Migration Path

### Phase 1: Add New Derive Macro

- Implement `LightCompressible` derive macro
- Parses `#[account(seeds = [...])]` from Anchor attribute
- Generates `XxxSeeds` structs
- Coexists with current `#[compressible(...)]`

### Phase 2: Update DecompressAccountsIdempotent Generation

- Auto-generate from parsed seeds across all Accounts structs
- Or use remaining_accounts approach
- Update `process_decompress_accounts_idempotent` to use new traits

### Phase 3: Simplify Module-Level Macro

- Reduce to just type list: `#[compressible_types(UserRecord, GameSession)]`
- Or remove entirely if types can be inferred

### Phase 4: Deprecate Old Syntax

- Emit warnings for old `#[compressible(Type = (seeds = ...))]` syntax
- Eventually remove

---

## Implementation Details

### Parsing Anchor Seeds Attribute

```rust
fn extract_anchor_seeds(field: &syn::Field) -> Option<Vec<SeedExpr>> {
    for attr in &field.attrs {
        if attr.path().is_ident("account") {
            // Parse: #[account(seeds = [...], bump, ...)]
            // Extract the seeds = [...] part
            // Return parsed seed expressions
        }
    }
    None
}

enum SeedExpr {
    Literal(Vec<u8>),           // b"user_record"
    AccountRef(Ident),          // authority.key().as_ref() -> authority
    ParamRef(Ident, Type),      // params.owner.as_ref() -> (owner, Pubkey)
    Constant(Path),             // MY_SEED
}
```

### Extracting Type from #[instruction]

```rust
#[derive(Accounts, LightCompressible)]
#[instruction(params: MyParams)]
pub struct CreateUserRecord<'info> { ... }

// Macro reads #[instruction(params: MyParams)]
// Then resolves MyParams to get field types for params.xxx references
```

This requires either:

1. The params type to be in the same module (can resolve)
2. User annotation: `#[compressible(params_type = MyParams)]`
3. Accept just the field name, infer type as Pubkey/u64/etc.

---

## Client-Side Changes

### Current

```rust
let decompress_account = CompressedAccountData {
    data: CompressedAccountVariant::UserRecord(packed_data),
    meta: compressed_account_meta,
};

// + SeedParams struct
// + named accounts in instruction
```

### Proposed

```rust
let decompress_input = DecompressInput {
    variant: CompressedAccountVariant::UserRecord(packed_data),
    seeds: UserRecordSeeds { authority, owner },  // Typed!
    compressed_account: compressed_account_data,
};

// Seeds struct is type-safe, IDE autocomplete works
```

---

## Open Questions

1. **Remaining accounts vs named optional accounts for decompress?**
   - Named: simpler, current approach, more readable
   - Remaining: more flexible, less struct bloat

2. ~~**How to handle token authority seeds?**~~ **RESOLVED**
   - Auto-detect from authority field's `#[account(seeds)]` or `Signer` type
   - Explicit `authority_seeds = (...)` as fallback
   - `authority_is_signer` for external signers
   - See "Token Authority Resolution" section

3. **Type inference for params.xxx references?**
   - Parse `#[instruction]` attribute for type
   - Or require explicit annotation
   - Or default to common types (Pubkey, u64)

4. **Enum generation without module-level macro?**
   - Scan all files for `#[compressible]` fields?
   - Explicit type list still needed?

---

## Benefits Summary

| Aspect                 | Current                   | Proposed             |
| ---------------------- | ------------------------- | -------------------- |
| Seed declarations      | 2 (Anchor + compressible) | 1 (Anchor only)      |
| Sync bugs possible     | Yes                       | No                   |
| Refactoring safety     | Low                       | High                 |
| Type-safe seed structs | Partial                   | Full                 |
| IDE support            | Limited                   | Better (typed seeds) |
| Maintenance burden     | High                      | Low                  |

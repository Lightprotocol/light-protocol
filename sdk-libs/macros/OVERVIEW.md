# `#[compressible]` Macro Usage Guide

## Supported Account Types

| Type                      | Description                                           |
| ------------------------- | ----------------------------------------------------- |
| **PDAs**                  | Program Derived Accounts with custom seeds            |
| **Program-owned CTokens** | Token accounts owned by a program PDA (vault pattern) |

---

## Program-Side: Macro Syntax

```rust
#[compressible(
    // PDA: TypeName = (seeds = (...))
    UserRecord = (seeds = ("user_record", ctx.authority, data.owner)),

    // Token: TypeName = (is_token, seeds = (...), authority = (...))
    Vault = (is_token, seeds = ("vault", ctx.mint), authority = ("vault_authority")),

    // Instruction data fields used in seeds
    owner = Pubkey,
)]
#[program]
pub mod my_program { ... }
```

### Seed Components

| Syntax           | Description                              |
| ---------------- | ---------------------------------------- |
| `seeds = (...)`  | Required. Tuple of seed elements         |
| `"literal"`      | Static seed bytes (string literal)       |
| `b"literal"`     | Static seed bytes (byte string literal)  |
| `CONST`          | Crate-level constant (`&str` or `&[u8]`) |
| `ctx.account`    | Account from instruction context         |
| `data.field`     | Field from instruction data              |
| `is_token`       | Marks account as CToken (not PDA)        |
| `authority = ()` | (tokens only) PDA that owns the token    |

**Constants:** Uppercase identifiers are resolved as `crate::CONST` and support both `&str` and `&[u8]`:

```rust
pub const MY_SEED: &str = "my_seed";        // &str constant
pub const MY_BYTES: &[u8] = b"my_bytes";    // &[u8] constant

#[compressible(
    MyAccount = (seeds = (MY_SEED, ctx.user)),
    MyOther = (seeds = (MY_BYTES, ctx.user)),
)]
```

---

## Generated Code

The macro generates:

1. **`CompressedAccountVariant`** - enum with all PDA types + token variants
2. **`TokenAccountVariant`** - enum for token account types
3. **`DecompressAccountsIdempotent`** - Anchor accounts struct
4. **`CompressAccountsIdempotent`** - Anchor accounts struct
5. **`SeedParams`** - struct for `data.*` seed fields
6. **`TokenSeedProvider`** impl - derives token seeds
7. **`PdaSeedDerivation`** impl - derives PDA seeds
8. **`DecompressContext`** impl - runtime decompression logic
9. **`decompress_accounts_idempotent()`** - instruction handler
10. **`compress_accounts_idempotent()`** - instruction handler

---

## Client-Side Usage

### Building Decompress Instruction

```rust
use light_compressible_client::compressible_instruction;

// 1. Fetch compressed accounts from indexer
let compressed_user = indexer.get_compressed_account(user_hash).await?;
let compressed_vault = indexer.get_compressed_account(vault_hash).await?;

// 2. Get validity proof
let proof = indexer.get_validity_proof(
    vec![compressed_user.hash, compressed_vault.hash],
    vec![],
    None,
).await?;

// 3. Build instruction
let instruction = compressible_instruction::decompress_accounts_idempotent(
    &program_id,
    &DECOMPRESS_DISCRIMINATOR,
    &[user_pda, vault_pda],           // Target on-chain addresses
    &[
        (compressed_user, user_data),   // PDAs first
        (compressed_vault, token_data), // Tokens after
    ],
    &program_accounts.to_account_metas(None),
    proof,
)?;

// 4. Append SeedParams if needed
let seed_params = SeedParams { owner };
instruction.data.extend_from_slice(&borsh::to_vec(&seed_params)?);
```

### Account Ordering

When mixing PDAs and tokens, order matters for CPI context:

```rust
// Correct: PDAs first, tokens after
&[
    (compressed_pda, pda_data),
    (compressed_token, token_data),
]
```

---

## CPI Context Rules

When decompressing **both PDAs and tokens** in one instruction:

1. PDAs **write** to CPI context first
2. Tokens **execute** (consume CPI context) last
3. CPI context validation checks: `cpi_context.associated_tree == first_input.tree` **at execution time**

**Critical:** The client uses the **first token's** `cpi_context`, not the first PDA's:

```rust
// In compressible-client (already handled internally):
// Uses first TOKEN's tree context since tokens execute last
let first_token_cpi_context = compressed_accounts
    .iter()
    .find(|(acc, _)| acc.owner == LIGHT_TOKEN_PROGRAM_ID)
    .map(|(acc, _)| acc.tree_info.cpi_context.unwrap());
```

---

## Example: Full Program

```rust
use anchor_lang::prelude::*;
use light_sdk_macros::compressible;

/// Seed constants - both &str and &[u8] are supported
pub const PROFILE_SEED: &str = "profile";
pub const VAULT_SEED: &[u8] = b"vault";

#[compressible(
    // PDA with &str constant
    UserProfile = (seeds = (PROFILE_SEED, ctx.authority, data.user_id)),

    // Token with &[u8] constant
    UserVault = (is_token, seeds = (VAULT_SEED, ctx.mint), authority = ("vault_auth", ctx.authority)),

    // Seed params
    user_id = [u8; 32],
)]
#[program]
pub mod my_program {
    use super::*;

    pub fn create_profile(ctx: Context<CreateProfile>, user_id: [u8; 32]) -> Result<()> {
        // ... create compressed profile
        Ok(())
    }

    // decompress_accounts_idempotent is auto-generated
    // compress_accounts_idempotent is auto-generated
}

#[derive(Accounts)]
pub struct CreateProfile<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    pub mint: Account<'info, Mint>,
}
```

---

## Key Files

| File                             | Purpose                         |
| -------------------------------- | ------------------------------- |
| `macros/src/compressible/`       | Macro implementation            |
| `sdk/src/compressible/`          | Runtime traits & PDA processing |
| `ctoken-sdk/src/compressible/`   | Token decompression runtime     |
| `compressible-client/src/lib.rs` | Client instruction builders     |

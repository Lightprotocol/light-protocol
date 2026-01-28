# Associated Token Account Documentation

## Overview

User associated token accounts (ATAs) for compressed tokens using `#[light_account([init,] associated_token::...)]`. ATAs are PDAs derived from the owner and mint addresses, providing a deterministic address for token storage.

Two modes are supported:
- **Init mode**: Creates the ATA using `CreateTokenAtaCpi` with idempotent() builder
- **Mark-only mode**: Marks existing ATA for derivation (used by `#[light_program]`)

## Two Modes

### Init Mode

```rust
#[light_account(init, associated_token, associated_token::authority = ..., associated_token::mint = ...)]
```

Creates the ATA using `CreateTokenAtaCpi` with idempotent() builder. The idempotent mode ensures the instruction succeeds even if the ATA already exists.

**Requirements:**
- `authority` - Required
- `mint` - Required
- `bump` - Optional (auto-derived if omitted)

### Mark-Only Mode

```rust
#[light_account(associated_token::authority = ..., associated_token::mint = ...)]
```

Marks an existing ATA for derivation. Used by `#[light_program]` for runtime PDA derivation. Returns `None` from parsing (skipped by LightAccounts derive).

**Requirements:**
- `authority` - Required (needed to derive ATA PDA at runtime)
- `mint` - Required (needed to derive ATA PDA at runtime)

Note: Unlike token accounts, mark-only mode also requires `mint` because both authority and mint are needed for ATA derivation.

## Parameters

| Parameter | Required | Mode | Description |
|-----------|----------|------|-------------|
| `associated_token::authority` | Yes | Both | Reference to the ATA owner field |
| `associated_token::mint` | Yes | Both | Reference to the mint field |
| `associated_token::bump` | No | Both | Explicit bump. If omitted, auto-derived via `derive_token_ata()` |

Note: `authority` is the user-facing parameter name but internally maps to the `owner` field of the ATA.

## Shorthand Syntax

All parameters support shorthand where the key alone means `key = key`:

```rust
// Shorthand
#[light_account(init, associated_token, associated_token::authority, associated_token::mint, associated_token::bump)]

// Equivalent to
#[light_account(init, associated_token, associated_token::authority = authority, associated_token::mint = mint, associated_token::bump = bump)]
```

## Validation Rules

1. `associated_token::authority` and `associated_token::mint` are always required in both modes
2. Unlike token accounts, mark-only mode also requires mint (needed for ATA derivation)
3. Bump is auto-derived if not provided using `derive_token_ata()`

## Infrastructure Requirements

The following infrastructure accounts must be present in the accounts struct when using init mode:

| Field Type | Accepted Names |
|------------|----------------|
| Fee Payer | `fee_payer`, `payer`, `creator` |
| Light Token Config | `light_token_compressible_config` |
| Light Token Rent Sponsor | `light_token_rent_sponsor`, `rent_sponsor` |
| Light Token Program | `light_token_program` |
| System Program | `system_program` |

## Examples

### Init Mode ATA

```rust
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateAtaParams)]
pub struct CreateAta<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Token mint
    pub mint: AccountInfo<'info>,

    /// CHECK: Owner of the ATA
    pub owner: AccountInfo<'info>,

    #[account(mut)]
    #[light_account(init, associated_token,
        associated_token::authority = owner,
        associated_token::mint = mint,
        associated_token::bump = params.ata_bump
    )]
    pub user_ata: UncheckedAccount<'info>,

    pub light_token_compressible_config: AccountInfo<'info>,
    #[account(mut)]
    pub light_token_rent_sponsor: AccountInfo<'info>,
    pub light_token_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}
```

### Init Mode with Shorthand

```rust
#[derive(Accounts, LightAccounts)]
#[instruction(bump: u8)]
pub struct CreateAtaShorthand<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Token mint
    pub mint: AccountInfo<'info>,

    /// CHECK: Owner of the ATA
    pub authority: AccountInfo<'info>,

    #[account(mut)]
    #[light_account(init, associated_token,
        associated_token::authority,
        associated_token::mint,
        associated_token::bump
    )]
    pub user_ata: UncheckedAccount<'info>,

    pub light_token_compressible_config: AccountInfo<'info>,
    #[account(mut)]
    pub light_token_rent_sponsor: AccountInfo<'info>,
    pub light_token_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}
```

### Mark-Only Mode

```rust
#[derive(Accounts, LightAccounts)]
pub struct TransferFromAta<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Token mint
    pub mint: AccountInfo<'info>,

    /// CHECK: Owner of the ATA
    pub owner: AccountInfo<'info>,

    #[light_account(associated_token::authority = owner, associated_token::mint = mint)]
    pub existing_ata: Account<'info, CToken>,
}
```

## Source References

- `sdk-libs/macros/src/light_pdas/accounts/token.rs` - ATA handling in `generate_ata_cpi`
- `sdk-libs/macros/src/light_pdas/light_account_keywords.rs` - `ASSOCIATED_TOKEN_NAMESPACE_KEYS`

## Related Documentation

- [architecture.md](./architecture.md) - Overall LightAccounts architecture
- [pda.md](./pda.md) - Compressed PDAs
- [mint.md](./mint.md) - Compressed mints
- [token.md](./token.md) - Token accounts (PDA-owned vaults)

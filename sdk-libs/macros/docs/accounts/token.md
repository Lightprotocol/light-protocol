# Token Account Attribute Documentation

## Overview

PDA-owned token accounts (vaults) using `#[light_account([init,] token::...)]`. This attribute enables the creation and management of token accounts that are owned by PDAs, commonly used for vault patterns in Solana programs.

There are two modes of operation:
- **Init mode**: Creates a new token account
- **Mark-only mode**: Marks an existing account for seed extraction (used by `#[light_program]` for decompress/compress instructions)

## Two Modes

### Init Mode

```rust
#[light_account(init, token, token::authority = [...], token::mint = ..., token::owner = ...)]
```

- Creates the token account
- Requires: `authority`, `mint`, `owner`
- Optional: `bump`

### Mark-Only Mode

```rust
#[light_account(token::authority = [...])]
```

- Marks existing account for seed derivation (used by `#[light_program]` for decompress/compress instructions)
- Returns `None` from parsing (skipped by LightAccounts derive)
- Requires: `authority` ONLY
- `mint` and `owner` are NOT allowed in mark-only mode

## Parameters

| Parameter | Required | Mode | Description |
|-----------|----------|------|-------------|
| `token::authority` | Yes | Both | PDA seeds for the token account authority (array expression like `[SEED, self.key.key()]`) |
| `token::mint` | Yes | init only | Reference to the mint field |
| `token::owner` | Yes | init only | Reference to the owner/authority PDA field |
| `token::bump` | No | Both | Explicit bump. If omitted, auto-derived via `find_program_address` |

## Shorthand Syntax

`mint`, `owner`, and `bump` support shorthand (key alone means `key = key`):

```rust
// Shorthand
#[light_account(init, token, token::authority = [...], token::mint, token::owner, token::bump)]

// Equivalent to
#[light_account(init, token, token::authority = [...], token::mint = mint, token::owner = owner, token::bump = bump)]
```

## Validation Rules

1. `token::authority` is always required
2. For init mode: `token::mint` and `token::owner` are required
3. For mark-only mode: `token::mint` and `token::owner` are NOT allowed
4. Empty authority seeds `[]` not allowed for init mode
5. Bump auto-derived if not provided

## Infrastructure Requirements

For init mode, the following infrastructure accounts are required in your accounts struct:

| Field Type | Accepted Names |
|------------|----------------|
| Fee Payer | `fee_payer`, `payer`, `creator` |
| Light Token Config | `light_token_compressible_config` |
| Light Token Rent Sponsor | `light_token_rent_sponsor`, `rent_sponsor` |
| Light Token Program | `light_token_program` |
| Light Token CPI Authority | `light_token_cpi_authority` |
| System Program | `system_program` |

## Examples

### Init Mode Vault

```rust
pub const VAULT_SEED: &[u8] = b"vault";
pub const VAULT_AUTH_SEED: &[u8] = b"vault_auth";

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateVaultParams)]
pub struct CreateVault<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Token mint
    pub mint: AccountInfo<'info>,

    #[account(seeds = [VAULT_AUTH_SEED], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [VAULT_SEED, mint.key().as_ref()],
        bump,
    )]
    #[light_account(init, token,
        token::authority = [VAULT_SEED, self.mint.key()],
        token::mint = mint,
        token::owner = vault_authority,
        token::bump = params.vault_bump
    )]
    pub vault: UncheckedAccount<'info>,

    pub light_token_compressible_config: AccountInfo<'info>,
    #[account(mut)]
    pub light_token_rent_sponsor: AccountInfo<'info>,
    pub light_token_cpi_authority: AccountInfo<'info>,
    pub light_token_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}
```

### Mark-Only Mode

Used when you need to reference an existing vault for seed extraction without initialization:

```rust
#[account(
    mut,
    seeds = [VAULT_SEED, mint.key().as_ref()],
    bump,
)]
#[light_account(token::authority = [VAULT_AUTH_SEED])]
pub vault: UncheckedAccount<'info>,
```

## Source References

- `sdk-libs/macros/src/light_pdas/accounts/token.rs` - Token account parsing and code generation
- `sdk-libs/macros/src/light_pdas/light_account_keywords.rs` - TOKEN_NAMESPACE_KEYS definitions

## Related Documentation

- [architecture.md](./architecture.md) - Overall LightAccounts architecture
- [pda.md](./pda.md) - Compressed PDAs
- [mint.md](./mint.md) - Compressed mints
- [associated_token.md](./associated_token.md) - Associated token accounts

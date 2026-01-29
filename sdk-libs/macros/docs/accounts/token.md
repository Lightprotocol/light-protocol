# Token Accounts

PDA-owned token accounts (vaults) using `token::` namespace parameters.

## Syntax

### Init Mode

Creates token account via `CreateTokenAccountCpi`.

```rust
#[light_account(init,
    token::seeds = [VAULT_SEED, self.mint.key()],
    token::mint = mint,
    token::owner = vault_authority,
    token::owner_seeds = [VAULT_AUTH_SEED],
    token::bump = params.vault_bump  // optional
)]
```

### Mark-Only Mode

Marks field for seed extraction. No account creation.

```rust
#[light_account(
    token::seeds = [VAULT_SEED, self.mint.key()],
    token::owner_seeds = [VAULT_AUTH_SEED]
)]
```

## Parameters

| Parameter | Init | Mark-Only | Description |
|-----------|------|-----------|-------------|
| `token::seeds` | Required | Required | Token account PDA seeds (no bump) |
| `token::owner_seeds` | Required | Required | Owner PDA seeds for decompression |
| `token::mint` | Required | Forbidden | Mint field reference |
| `token::owner` | Required | Forbidden | Owner/authority field reference |
| `token::bump` | Optional | Optional | Explicit bump, auto-derived if omitted |

## Validation

**Init mode:**
- All of seeds, owner_seeds, mint, owner required
- Empty seeds forbidden

**Mark-only mode:**
- Only seeds and owner_seeds permitted
- mint and owner forbidden

## Infrastructure (init mode)

| Field | Names |
|-------|-------|
| Fee payer | `fee_payer`, `payer`, `creator` |
| Config | `light_token_config` |
| Rent sponsor | `light_token_rent_sponsor` |
| CPI authority | `light_token_cpi_authority` |
| Token program | `light_token_program` |
| System program | `system_program` |

## Example

```rust
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateVaultParams)]
pub struct CreateVault<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub mint: AccountInfo<'info>,
    #[account(seeds = [VAULT_AUTH_SEED], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(mut, seeds = [VAULT_SEED, mint.key().as_ref()], bump)]
    #[light_account(init,
        token::seeds = [VAULT_SEED, self.mint.key()],
        token::mint = mint,
        token::owner = vault_authority,
        token::owner_seeds = [VAULT_AUTH_SEED],
        token::bump = params.vault_bump
    )]
    pub vault: UncheckedAccount<'info>,

    pub light_token_config: AccountInfo<'info>,
    #[account(mut)]
    pub light_token_rent_sponsor: AccountInfo<'info>,
    pub light_token_cpi_authority: AccountInfo<'info>,
    pub light_token_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}
```

## Source

- `sdk-libs/macros/src/light_pdas/accounts/token.rs` - CPI generation
- `sdk-libs/macros/src/light_pdas/accounts/light_account.rs` - Parsing (lines 109-123, 882-1021)
- `sdk-libs/macros/src/light_pdas/light_account_keywords.rs` - TOKEN_NAMESPACE_KEYS

## Related

- [architecture.md](./architecture.md)
- [associated_token.md](./associated_token.md)

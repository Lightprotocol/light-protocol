# Token Accounts

PDA-owned token accounts (vaults) using `token::` namespace parameters.

## Syntax

### Init Mode

Creates token account via `CreateTokenAccountCpi` in `LightPreInit`.

```rust
#[light_account(init,
    token::seeds = [VAULT_SEED, self.mint.key()],      // Token account PDA seeds (WITHOUT bump)
    token::owner_seeds = [VAULT_AUTH_SEED],            // Owner PDA seeds (WITHOUT bump)
    token::mint = mint,                                // Mint field reference
    token::owner = vault_authority,                    // Owner field reference
    token::bump = params.vault_bump                    // Optional: explicit bump
)]
pub vault: Account<'info, CToken>,
```

### Mark-Only Mode

Marks field for seed extraction. No account creation. Used by `#[light_program]` for compress/decompress instruction generation.

```rust
#[light_account(
    token::seeds = [VAULT_SEED, self.mint.key()],      // Token account PDA seeds (WITHOUT bump)
    token::owner_seeds = [VAULT_AUTH_SEED]             // Owner PDA seeds (WITHOUT bump)
)]
pub vault: Account<'info, CToken>,
```

## Parameters

| Parameter | Init | Mark-Only | Description |
|-----------|------|-----------|-------------|
| `token::seeds` | Required | Required | Token account PDA seeds (WITHOUT bump - bump is added automatically) |
| `token::owner_seeds` | Required | Required | Owner PDA seeds for decompression (WITHOUT bump) |
| `token::mint` | Required | Forbidden | Mint field reference |
| `token::owner` | Required | Forbidden | Owner/authority field reference |
| `token::bump` | Optional | Optional | Explicit bump for token::seeds (auto-derived using `find_program_address` if omitted) |

**Seed handling:**
- User provides base seeds WITHOUT bump in `token::seeds` array
- Macro auto-derives bump using `Pubkey::find_program_address()` if `token::bump` not provided
- Bump is always appended as the final seed when calling `invoke_signed()`

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
const VAULT_SEED: &[u8] = b"vault";
const VAULT_AUTH_SEED: &[u8] = b"vault_authority";

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateVaultParams)]
pub struct CreateVault<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub mint: AccountInfo<'info>,

    #[account(seeds = [VAULT_AUTH_SEED], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    // Token account with init - creates via CreateTokenAccountCpi in pre_init
    #[light_account(init,
        token::seeds = [VAULT_SEED, self.mint.key()],      // Token account PDA seeds (no bump)
        token::owner_seeds = [VAULT_AUTH_SEED],            // Owner PDA seeds (no bump)
        token::mint = mint,                                // Mint field reference
        token::owner = vault_authority,                    // Owner field reference
        token::bump = params.vault_bump                    // Optional bump
    )]
    pub vault: Account<'info, CToken>,

    // Infrastructure for token account creation
    pub light_token_config: Account<'info, CompressibleConfig>,
    #[account(mut)]
    pub light_token_rent_sponsor: Account<'info, RentSponsor>,
    pub light_token_cpi_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}
```

### Generated Code

The macro generates `CreateTokenAccountCpi` call in `LightPreInit::light_pre_init()`:

```rust
impl<'info> LightPreInit<'info, CreateVaultParams> for CreateVault<'info> {
    fn light_pre_init(&mut self, _remaining: &[AccountInfo<'info>], params: &CreateVaultParams)
        -> Result<bool, LightSdkError>
    {
        // Bind seeds to local variables (extends temporary lifetimes)
        let __seed_0 = VAULT_SEED;
        let __seed_0_ref: &[u8] = __seed_0.as_ref();
        let __seed_1 = self.mint.key();
        let __seed_1_ref: &[u8] = __seed_1.as_ref();

        // Get bump - either provided or auto-derived
        let __bump: u8 = params.vault_bump;  // or auto-derive if not provided
        let __bump_slice: [u8; 1] = [__bump];
        let __token_account_seeds: &[&[u8]] = &[__seed_0_ref, __seed_1_ref, &__bump_slice[..]];

        CreateTokenAccountCpi {
            payer: self.fee_payer.to_account_info(),
            account: self.vault.to_account_info(),
            mint: self.mint.to_account_info(),
            owner: *self.vault_authority.to_account_info().key,
        }
        .rent_free(
            self.light_token_config.to_account_info(),
            self.light_token_rent_sponsor.to_account_info(),
            __system_program.clone(),
            &crate::ID,
        )
        .invoke_signed(__token_account_seeds)?;

        Ok(true)
    }
}
```

## Requirements

Programs using token account creation must:
- Define `crate::ID` constant (standard with Anchor's `declare_id!`)
- Include `system_program` field in the accounts struct
- The generated code uses `system_program` for token account creation via CPI

## Source

- `sdk-libs/macros/src/light_pdas/accounts/token.rs` - CPI generation
- `sdk-libs/macros/src/light_pdas/accounts/light_account.rs` - Parsing
- `sdk-libs/macros/src/light_pdas/light_account_keywords.rs` - TOKEN_NAMESPACE_KEYS
- `sdk-libs/macros/src/light_pdas/accounts/builder.rs` - Pre-init code generation

## Related

- [architecture.md](./architecture.md)
- [associated_token.md](./associated_token.md)

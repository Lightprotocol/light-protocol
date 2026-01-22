# `#[light_account(init, mint::...)]` Attribute

## Overview

The `#[light_account(init, mint::...)]` attribute marks a field in an Anchor Accounts struct for compressed mint creation. When applied to a `Mint` account field, it generates code to create a compressed mint with automatic decompression support.

**Source**: `sdk-libs/macros/src/light_pdas/accounts/light_account.rs`

## Syntax

All parameters use the Anchor-style `mint::` namespace prefix. The account type is inferred from the namespace:

```rust
#[light_account(init,
    mint::signer = mint_signer,
    mint::authority = authority,
    mint::decimals = 9,
    mint::seeds = &[b"mint_signer", &[ctx.bumps.mint_signer]]
)]
pub mint: UncheckedAccount<'info>,
```

## Usage

```rust
use light_sdk_macros::LightAccounts;
use anchor_lang::prelude::*;

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateParams)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Unchecked account for PDA signer
    #[account(seeds = [b"mint_signer"], bump)]
    pub mint_signer: AccountInfo<'info>,

    pub authority: Signer<'info>,

    /// The Mint account to create
    #[light_account(init,
        mint::signer = mint_signer,
        mint::authority = authority,
        mint::decimals = 9,
        mint::seeds = &[b"mint_signer", &[ctx.bumps.mint_signer]]
    )]
    pub mint: UncheckedAccount<'info>,

    // Infrastructure accounts (auto-detected by name)
    pub light_token_compressible_config: Account<'info, CtokenConfig>,
    pub ctoken_rent_sponsor: Account<'info, CtokenRentSponsor>,
    pub light_token_program: Program<'info, LightTokenProgram>,
    pub light_token_cpi_authority: AccountInfo<'info>,
}
```

## Required Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `mint::signer` | Field reference | The AccountInfo that seeds the mint PDA. The mint address is derived from this signer. |
| `mint::authority` | Field reference | The mint authority. Either a transaction signer or a PDA (if `mint::authority_seeds` is provided). |
| `mint::decimals` | Expression | Token decimals (e.g., `9` for 9 decimal places). |
| `mint::seeds` | Slice expression | PDA signer seeds for `mint_signer`. Must be a `&[&[u8]]` expression that matches the `#[account(seeds = ...)]` on `mint_signer`, **including the bump**. |

## Optional Attributes

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `mint::bump` | Expression | Auto-derived | Explicit bump seed for the mint signer PDA. If not provided, uses `find_program_address`. |
| `mint::freeze_authority` | Field reference | None | Optional freeze authority field. |
| `mint::authority_seeds` | Slice expression | None | PDA signer seeds for `authority`. If not provided, `authority` must be a transaction signer. |
| `mint::authority_bump` | Expression | Auto-derived | Explicit bump seed for authority PDA. |
| `mint::rent_payment` | Expression | `2u8` | Rent payment epochs for decompression. |
| `mint::write_top_up` | Expression | `0u32` | Write top-up lamports for decompression. |

## TokenMetadata Fields

Optional fields for creating a mint with the TokenMetadata extension:

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `mint::name` | Expression | - | Token name (expression yielding `Vec<u8>`). |
| `mint::symbol` | Expression | - | Token symbol (expression yielding `Vec<u8>`). |
| `mint::uri` | Expression | - | Token URI (expression yielding `Vec<u8>`). |
| `mint::update_authority` | Field reference | None | Optional update authority for metadata. |
| `mint::additional_metadata` | Expression | None | Additional key-value metadata (expression yielding `Option<Vec<AdditionalMetadata>>`). |

### Validation Rules

1. **Core fields are all-or-nothing**: `mint::name`, `mint::symbol`, and `mint::uri` must ALL be specified together, or none at all.
2. **Optional fields require core fields**: `mint::update_authority` and `mint::additional_metadata` require `mint::name`, `mint::symbol`, and `mint::uri` to also be specified.

### Metadata Example

```rust
#[light_account(init,
    mint::signer = mint_signer,
    mint::authority = fee_payer,
    mint::decimals = 9,
    mint::seeds = &[SEED, self.authority.key().as_ref(), &[params.bump]],
    // TokenMetadata fields
    mint::name = params.name.clone(),
    mint::symbol = params.symbol.clone(),
    mint::uri = params.uri.clone(),
    mint::update_authority = authority,
    mint::additional_metadata = params.additional_metadata.clone()
)]
pub mint: UncheckedAccount<'info>,
```

**Invalid configurations (compile-time errors):**

```rust
// ERROR: name without symbol and uri
#[light_account(init,
    mint::signer = ...,
    mint::name = params.name.clone()
)]

// ERROR: additional_metadata without name, symbol, uri
#[light_account(init,
    mint::signer = ...,
    mint::additional_metadata = params.additional_metadata.clone()
)]
```

## How It Works

### Mint PDA Derivation

The mint address is derived from the `mint_signer` field:

```rust
let (mint_pda, bump) = light_token::instruction::find_mint_address(mint_signer.key);
```

### Signer Seeds (mint::seeds)

The `mint::seeds` attribute provides the PDA signer seeds used for `invoke_signed` when calling the light token program. These seeds must derive to the `mint_signer` pubkey for the CPI to succeed.

```rust
#[light_account(init,
    mint::signer = mint_signer,
    mint::authority = mint_authority,
    mint::decimals = 9,
    mint::seeds = &[LP_MINT_SIGNER_SEED, self.authority.to_account_info().key.as_ref(), &[params.mint_signer_bump]],
    mint::bump = params.mint_signer_bump
)]
pub mint: UncheckedAccount<'info>,
```

**Syntax notes:**
- Use `self.field` to reference accounts in the struct
- Use `.to_account_info().key` to get account pubkeys
- The bump can be provided explicitly via `mint::bump` or auto-derived

The generated code uses these seeds to sign the CPI:

```rust
let mint_seeds: &[&[u8]] = &[...]; // from mint::seeds attribute
invoke_signed(&mint_action_ix, &account_infos, &[mint_seeds])?;
```

### Generated Code Flow

1. **Resolve tree accounts** - Get address tree and output queue from CPI accounts
2. **Derive mint PDA** - Calculate mint address from `mint_signer`
3. **Extract proof** - Get compression proof from instruction params
4. **Build mint instruction data** - Create `MintInstructionData` with metadata
5. **Configure decompression** - Set `rent_payment` and `write_top_up` for decompression
6. **Build account metas** - Configure CPI accounts for mint_action
7. **Invoke CPI** - Call light_token_program with signer seeds

### CPI Context Integration

When used alongside `#[light_account(init)]` PDAs, the mint is batched with PDA compression in a single CPI context. The mint receives an `assigned_account_index` to order it relative to PDAs.

## Examples

### Basic Mint Creation

```rust
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateParams)]
pub struct CreateBasicMint<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Mint signer PDA
    #[account(seeds = [b"mint"], bump)]
    pub mint_signer: AccountInfo<'info>,

    pub authority: Signer<'info>,

    #[light_account(init,
        mint::signer = mint_signer,
        mint::authority = authority,
        mint::decimals = 6,
        mint::seeds = &[b"mint", &[ctx.bumps.mint_signer]]
    )]
    pub mint: UncheckedAccount<'info>,

    // ... infrastructure accounts
}
```

### Mint with PDA Authority

When the authority is a PDA, provide `mint::authority_seeds`:

```rust
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateParams)]
pub struct CreateMintWithPdaAuthority<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Mint signer PDA
    #[account(seeds = [b"mint"], bump)]
    pub mint_signer: AccountInfo<'info>,

    /// CHECK: Authority PDA (not a signer)
    #[account(seeds = [b"authority"], bump)]
    pub authority: AccountInfo<'info>,

    #[light_account(init,
        mint::signer = mint_signer,
        mint::authority = authority,
        mint::decimals = 9,
        mint::seeds = &[b"mint", &[ctx.bumps.mint_signer]],
        mint::authority_seeds = &[b"authority", &[ctx.bumps.authority]],
        mint::authority_bump = params.authority_bump
    )]
    pub mint: UncheckedAccount<'info>,

    // ... infrastructure accounts
}
```

### Mint with Freeze Authority

```rust
#[light_account(init,
    mint::signer = mint_signer,
    mint::authority = authority,
    mint::decimals = 9,
    mint::seeds = &[b"mint", &[bump]],
    mint::freeze_authority = freeze_auth
)]
pub mint: UncheckedAccount<'info>,

/// Optional freeze authority
pub freeze_auth: Signer<'info>,
```

### Custom Decompression Settings

```rust
#[light_account(init,
    mint::signer = mint_signer,
    mint::authority = authority,
    mint::decimals = 9,
    mint::seeds = &[b"mint", &[bump]],
    mint::rent_payment = 4,      // 4 epochs of rent
    mint::write_top_up = 1000    // Extra lamports for writes
)]
pub mint: UncheckedAccount<'info>,
```

### Combined with #[light_account(init)] PDAs

```rust
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateParams)]
pub struct CreateMintAndPda<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Mint signer
    #[account(seeds = [b"mint"], bump)]
    pub mint_signer: AccountInfo<'info>,

    pub authority: Signer<'info>,

    #[light_account(init,
        mint::signer = mint_signer,
        mint::authority = authority,
        mint::decimals = 9,
        mint::seeds = &[b"mint", &[ctx.bumps.mint_signer]]
    )]
    pub mint: UncheckedAccount<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + TokenAccount::INIT_SPACE,
        seeds = [b"token", params.owner.as_ref()],
        bump
    )]
    #[light_account(init)]
    pub token_account: Account<'info, TokenAccount>,

    // ... infrastructure accounts
}
```

When both `#[light_account(init)]` and `#[light_account(init, mint::...)]` are present, the macro:
1. Processes PDAs first, writing them to the CPI context
2. Invokes mint_action with CPI context to batch the mint creation
3. Uses `assigned_account_index` to order the mint relative to PDAs

## Infrastructure Accounts

The macro requires certain infrastructure accounts, auto-detected by naming convention:

| Account Type | Accepted Names |
|--------------|----------------|
| Fee Payer | `fee_payer`, `payer`, `creator` |
| CToken Config | `light_token_compressible_config`, `ctoken_config`, `light_token_config_account` |
| CToken Rent Sponsor | `ctoken_rent_sponsor`, `light_token_rent_sponsor` |
| CToken Program | `ctoken_program`, `light_token_program` |
| CToken CPI Authority | `light_token_cpi_authority`, `light_token_program_cpi_authority`, `compress_token_program_cpi_authority` |

## Validation

The macro validates at compile time:
- `mint::signer`, `mint::authority`, `mint::decimals`, and `mint::seeds` are required
- `#[instruction(...)]` attribute must be present on the struct
- If `mint::authority_seeds` is not provided, the generated code verifies `authority` is a transaction signer at runtime

## Related Documentation

- **`../CLAUDE.md`** - Main entry point for sdk-libs/macros
- **`../light_program/`** - Program-level `#[light_program]` macro
- **`../account/`** - Trait derives for data structs

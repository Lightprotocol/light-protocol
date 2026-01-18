# `#[light_mint(...)]` Attribute

## Overview

The `#[light_mint(...)]` attribute marks a field in an Anchor Accounts struct for compressed mint creation. When applied to a `CMint` account field, it generates code to create a compressed mint with automatic decompression support.

**Source**: `sdk-libs/macros/src/rentfree/accounts/light_mint.rs`

## Usage

```rust
use light_sdk_macros::RentFree;
use anchor_lang::prelude::*;

#[derive(Accounts, RentFree)]
#[instruction(params: CreateParams)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Unchecked account for PDA signer
    #[account(seeds = [b"mint_signer"], bump)]
    pub mint_signer: AccountInfo<'info>,

    pub authority: Signer<'info>,

    /// The CMint account to create
    #[light_mint(
        mint_signer = mint_signer,
        authority = authority,
        decimals = 9,
        mint_seeds = &[b"mint_signer", &[ctx.bumps.mint_signer]]
    )]
    pub cmint: Account<'info, CMint>,

    // Infrastructure accounts (auto-detected by name)
    pub ctoken_compressible_config: Account<'info, CtokenConfig>,
    pub ctoken_rent_sponsor: Account<'info, CtokenRentSponsor>,
    pub light_token_program: Program<'info, LightTokenProgram>,
    pub ctoken_cpi_authority: AccountInfo<'info>,
}
```

## Required Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `mint_signer` | Field reference | The AccountInfo that seeds the mint PDA. The mint address is derived from this signer. |
| `authority` | Field reference | The mint authority. Either a transaction signer or a PDA (if `authority_seeds` is provided). |
| `decimals` | Expression | Token decimals (e.g., `9` for 9 decimal places). |
| `mint_seeds` | Slice expression | PDA signer seeds for `mint_signer`. Must be a `&[&[u8]]` expression that matches the `#[account(seeds = ...)]` on `mint_signer`, **including the bump**. |

## Optional Attributes

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `address_tree_info` | Expression | `params.create_accounts_proof.address_tree_info` | `PackedAddressTreeInfo` containing tree indices. |
| `freeze_authority` | Field reference | None | Optional freeze authority field. |
| `authority_seeds` | Slice expression | None | PDA signer seeds for `authority`. If not provided, `authority` must be a transaction signer. |
| `rent_payment` | Expression | `2u8` | Rent payment epochs for decompression. |
| `write_top_up` | Expression | `0u32` | Write top-up lamports for decompression. |

## How It Works

### Mint PDA Derivation

The mint address is derived from the `mint_signer` field:

```rust
let (mint_pda, bump) = light_token_sdk::token::find_mint_address(mint_signer.key);
```

### Signer Seeds (mint_seeds)

The `mint_seeds` attribute provides the PDA signer seeds used for `invoke_signed` when calling the light token program. These seeds must derive to the `mint_signer` pubkey for the CPI to succeed.

```rust
#[light_mint(
    mint_signer = mint_signer,
    authority = mint_authority,
    decimals = 9,
    mint_seeds = &[LP_MINT_SIGNER_SEED, self.authority.to_account_info().key.as_ref(), &[params.mint_signer_bump]]
)]
pub cmint: UncheckedAccount<'info>,
```

**Syntax notes:**
- Use `self.field` to reference accounts in the struct
- Use `.to_account_info().key` to get account pubkeys
- The bump must be passed explicitly (typically via instruction params)

The generated code uses these seeds to sign the CPI:

```rust
let mint_seeds: &[&[u8]] = &[...]; // from mint_seeds attribute
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

When used alongside `#[rentfree]` PDAs, the mint is batched with PDA compression in a single CPI context. The mint receives an `assigned_account_index` to order it relative to PDAs.

## Examples

### Basic Mint Creation

```rust
#[derive(Accounts, RentFree)]
#[instruction(params: CreateParams)]
pub struct CreateBasicMint<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Mint signer PDA
    #[account(seeds = [b"mint"], bump)]
    pub mint_signer: AccountInfo<'info>,

    pub authority: Signer<'info>,

    #[light_mint(
        mint_signer = mint_signer,
        authority = authority,
        decimals = 6,
        mint_seeds = &[b"mint", &[ctx.bumps.mint_signer]]
    )]
    pub cmint: Account<'info, CMint>,

    // ... infrastructure accounts
}
```

### Mint with PDA Authority

When the authority is a PDA, provide `authority_seeds`:

```rust
#[derive(Accounts, RentFree)]
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

    #[light_mint(
        mint_signer = mint_signer,
        authority = authority,
        decimals = 9,
        mint_seeds = &[b"mint", &[ctx.bumps.mint_signer]],
        authority_seeds = &[b"authority", &[ctx.bumps.authority]]
    )]
    pub cmint: Account<'info, CMint>,

    // ... infrastructure accounts
}
```

### Mint with Freeze Authority

```rust
#[light_mint(
    mint_signer = mint_signer,
    authority = authority,
    decimals = 9,
    mint_seeds = &[b"mint", &[bump]],
    freeze_authority = freeze_auth
)]
pub cmint: Account<'info, CMint>,

/// Optional freeze authority
pub freeze_auth: Signer<'info>,
```

### Custom Decompression Settings

```rust
#[light_mint(
    mint_signer = mint_signer,
    authority = authority,
    decimals = 9,
    mint_seeds = &[b"mint", &[bump]],
    rent_payment = 4,      // 4 epochs of rent
    write_top_up = 1000    // Extra lamports for writes
)]
pub cmint: Account<'info, CMint>,
```

### Combined with #[rentfree] PDAs

```rust
#[derive(Accounts, RentFree)]
#[instruction(params: CreateParams)]
pub struct CreateMintAndPda<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Mint signer
    #[account(seeds = [b"mint"], bump)]
    pub mint_signer: AccountInfo<'info>,

    pub authority: Signer<'info>,

    #[light_mint(
        mint_signer = mint_signer,
        authority = authority,
        decimals = 9,
        mint_seeds = &[b"mint", &[ctx.bumps.mint_signer]]
    )]
    pub cmint: Account<'info, CMint>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + TokenAccount::INIT_SPACE,
        seeds = [b"token", params.owner.as_ref()],
        bump
    )]
    #[rentfree]
    pub token_account: Account<'info, TokenAccount>,

    // ... infrastructure accounts
}
```

When both `#[light_mint]` and `#[rentfree]` are present, the macro:
1. Processes PDAs first, writing them to the CPI context
2. Invokes mint_action with CPI context to batch the mint creation
3. Uses `assigned_account_index` to order the mint relative to PDAs

## Infrastructure Accounts

The macro requires certain infrastructure accounts, auto-detected by naming convention:

| Account Type | Accepted Names |
|--------------|----------------|
| Fee Payer | `fee_payer`, `payer`, `creator` |
| CToken Config | `ctoken_compressible_config`, `ctoken_config`, `light_token_config_account` |
| CToken Rent Sponsor | `ctoken_rent_sponsor`, `light_token_rent_sponsor` |
| CToken Program | `ctoken_program`, `light_token_program` |
| CToken CPI Authority | `ctoken_cpi_authority`, `light_token_program_cpi_authority`, `compress_token_program_cpi_authority` |

## Validation

The macro validates at compile time:
- `mint_signer`, `authority`, `decimals`, and `mint_seeds` are required
- `#[instruction(...)]` attribute must be present on the struct
- If `authority_seeds` is not provided, the generated code verifies `authority` is a transaction signer at runtime

## Related Documentation

- **`../rentfree.md`** - Full RentFree derive macro documentation
- **`../rentfree_program/`** - Program-level `#[rentfree_program]` macro
- **`../account/`** - Trait derives for data structs

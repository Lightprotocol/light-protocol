# Compressed Mint Creation with `#[light_account(init, mint::...)]`

## Overview

Compressed mint creation uses `#[light_account(init, mint::...)]` to create compressed mints with automatic address registration and optional TokenMetadata extension for embedded metadata (name, symbol, URI).

The mint address is derived from a signer AccountInfo using `find_mint_address()`. Tree info is automatically fetched from `CreateAccountsProof` in the instruction parameters.

**Source**: `sdk-libs/macros/src/light_pdas/accounts/mint.rs`

---

## Required Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `mint::signer` | Field reference | AccountInfo that seeds the mint PDA. The mint address is derived from this signer using `find_mint_address()`. |
| `mint::authority` | Field reference | Mint authority. Either a transaction signer or a PDA (if `mint::authority_seeds` provided). |
| `mint::decimals` | Expression | Token decimals (e.g., `9`). |
| `mint::seeds` | Slice expression | Base PDA signer seeds for `mint_signer` (WITHOUT bump - bump is auto-derived or provided via `mint::bump`). |

---

## Optional Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `mint::bump` | Expression | Auto-derived | Bump for mint_signer PDA. If omitted, derived using `find_program_address`. |
| `mint::freeze_authority` | Field reference | None | Optional freeze authority field. |
| `mint::authority_seeds` | Slice expression | None | PDA seeds if authority is a PDA (without bump). |
| `mint::authority_bump` | Expression | Auto-derived | Bump for authority_seeds. |
| `mint::rent_payment` | Expression | `16u8` | Decompression rent payment epochs. |
| `mint::write_top_up` | Expression | `766u32` | Decompression write top-up lamports. |

---

## TokenMetadata Extension Parameters

The TokenMetadata extension allows embedding metadata directly in the compressed mint. This follows an **all-or-nothing rule**: `name`, `symbol`, and `uri` must ALL be specified together, or none at all.

### Core Metadata Fields

| Parameter | Type | Description |
|-----------|------|-------------|
| `mint::name` | Expression | Token name. Must yield `Vec<u8>`. |
| `mint::symbol` | Expression | Token symbol. Must yield `Vec<u8>`. |
| `mint::uri` | Expression | Token URI. Must yield `Vec<u8>`. |

### Optional Metadata Fields

These require the core metadata fields (`name`, `symbol`, `uri`) to be present:

| Parameter | Type | Description |
|-----------|------|-------------|
| `mint::update_authority` | Field reference | Metadata update authority field. |
| `mint::additional_metadata` | Expression | Additional metadata key-value pairs. Must yield `Option<Vec<AdditionalMetadata>>`. |

---

## Validation Rules

1. **Required fields**: `mint::signer`, `mint::authority`, `mint::decimals`, `mint::seeds` must all be specified.

2. **TokenMetadata all-or-nothing**: `name`, `symbol`, and `uri` must all be specified together, or none at all. Specifying only some causes a compile error.

3. **Optional metadata requires core**: `update_authority` and `additional_metadata` require `name`, `symbol`, and `uri` to be present.

4. **Authority signer check**: If `authority_seeds` is not provided, the authority must be a transaction signer. This is checked at runtime with `MissingRequiredSignature` error.

---

## Infrastructure Requirements

The macro auto-detects infrastructure fields by naming convention:

| Field Type | Accepted Names |
|------------|----------------|
| Fee Payer | `fee_payer`, `payer`, `creator` |
| Light Token Config | `light_token_compressible_config` |
| Light Token Rent Sponsor | `light_token_rent_sponsor`, `rent_sponsor` |
| Light Token Program | `light_token_program` |
| Light Token CPI Authority | `light_token_cpi_authority` |

---

## Examples

### Basic Mint

Creates a compressed mint with minimal configuration:

```rust
pub const MINT_SEED: &[u8] = b"mint";

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateMintParams)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Authority for the mint
    pub authority: Signer<'info>,

    /// CHECK: Seeds the mint PDA
    #[account(seeds = [MINT_SEED], bump)]
    pub mint_signer: AccountInfo<'info>,

    #[light_account(init,
        mint::signer = mint_signer,
        mint::authority = authority,
        mint::decimals = 6,
        mint::seeds = &[MINT_SEED]
    )]
    pub mint: UncheckedAccount<'info>,

    // Infrastructure accounts
    #[account(address = COMPRESSIBLE_CONFIG)]
    pub light_token_compressible_config: AccountInfo<'info>,

    #[account(mut, address = RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    pub light_token_cpi_authority: AccountInfo<'info>,
    pub light_token_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}
```

### Mint with PDA Authority

When the mint authority is a PDA rather than a signer:

```rust
pub const MINT_SEED: &[u8] = b"mint";
pub const AUTHORITY_SEED: &[u8] = b"authority";

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateMintParams)]
pub struct CreateMintWithPdaAuthority<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: PDA authority for the mint
    #[account(seeds = [AUTHORITY_SEED], bump)]
    pub authority: AccountInfo<'info>,

    /// CHECK: Seeds the mint PDA
    #[account(seeds = [MINT_SEED], bump)]
    pub mint_signer: AccountInfo<'info>,

    #[light_account(init,
        mint::signer = mint_signer,
        mint::authority = authority,
        mint::decimals = 9,
        mint::seeds = &[MINT_SEED],
        mint::authority_seeds = &[AUTHORITY_SEED],
        mint::authority_bump = params.authority_bump
    )]
    pub mint: UncheckedAccount<'info>,

    // Infrastructure accounts...
}
```

### Mint with TokenMetadata Extension

Creates a compressed mint with embedded metadata:

```rust
pub const SEED: &[u8] = b"mint";

#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateMintWithMetadataParams)]
pub struct CreateMintWithMetadata<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Authority for the mint and metadata
    pub authority: Signer<'info>,

    /// CHECK: Seeds the mint PDA
    #[account(seeds = [SEED, authority.key().as_ref()], bump)]
    pub mint_signer: AccountInfo<'info>,

    #[light_account(init,
        mint::signer = mint_signer,
        mint::authority = fee_payer,
        mint::decimals = 9,
        mint::seeds = &[SEED, self.authority.to_account_info().key.as_ref()],
        mint::bump = params.mint_signer_bump,
        mint::name = params.name.clone(),
        mint::symbol = params.symbol.clone(),
        mint::uri = params.uri.clone(),
        mint::update_authority = authority,
        mint::additional_metadata = params.additional_metadata.clone()
    )]
    pub mint: UncheckedAccount<'info>,

    // Infrastructure accounts...
}
```

### Mint with Freeze Authority

```rust
#[light_account(init,
    mint::signer = mint_signer,
    mint::authority = authority,
    mint::decimals = 6,
    mint::seeds = &[b"mint"],
    mint::freeze_authority = freeze_auth
)]
pub mint: UncheckedAccount<'info>,
```

### Mint with Custom Rent Settings

```rust
#[light_account(init,
    mint::signer = mint_signer,
    mint::authority = authority,
    mint::decimals = 6,
    mint::seeds = &[b"mint"],
    mint::rent_payment = 32u8,       // Custom rent payment epochs
    mint::write_top_up = 1000u32     // Custom write top-up lamports
)]
pub mint: UncheckedAccount<'info>,
```

---

## Generated Code

The macro generates code that:

1. **Derives the mint PDA** using `light_token::instruction::find_mint_address()` from the mint_signer key
2. **Builds signer seeds** with the bump appended (auto-derived or provided)
3. **Constructs `SingleMintParams`** with all mint configuration
4. **Builds `TokenMetadataInstructionData`** if metadata fields are provided
5. **Invokes `CreateMintsCpi`** via `light_token::compressible::invoke_create_mints()`

### Key Code Flow

```rust
// 1. Get mint signer key and derive mint address
let signer_key = *self.mint_signer.to_account_info().key;
let (mint_pda, mint_bump) = light_token::instruction::find_mint_address(&signer_key);

// 2. Build signer seeds with bump
let mint_seeds: &[&[u8]] = &[MINT_SEED];
let mint_signer_bump = params.mint_signer_bump;  // or auto-derived
let mut mint_seeds_with_bump = mint_seeds.to_vec();
mint_seeds_with_bump.push(&[mint_signer_bump]);

// 3. Build SingleMintParams
let mint_param = SingleMintParams {
    decimals: 9,
    address_merkle_tree_root_index: tree_info.root_index,
    mint_authority: *self.authority.key,
    compression_address: mint_pda.to_bytes(),
    mint: mint_pda,
    bump: mint_bump,
    freeze_authority: None,
    mint_seed_pubkey: signer_key,
    authority_seeds: None,  // or Some(...) if PDA authority
    mint_signer_seeds: Some(&mint_seeds_with_bump[..]),
    token_metadata: metadata.as_ref(),  // or None
};

// 4. Invoke CreateMintsCpi
light_token::compressible::invoke_create_mints(
    &[mint_signer_account_info],
    &[mint_account_info],
    CreateMintsParams { mints: &[mint_param], ... },
    CreateMintsInfraAccounts { ... },
    &cpi_accounts,
)?;
```

---

## Source References

- **Mint code generation**: `sdk-libs/macros/src/light_pdas/accounts/mint.rs`
- **Keyword definitions**: `sdk-libs/macros/src/light_pdas/light_account_keywords.rs` (`MINT_NAMESPACE_KEYS`)
- **Attribute parsing**: `sdk-libs/macros/src/light_pdas/accounts/light_account.rs`
- **Light Token types**: `light_token::instruction::SingleMintParams`, `CreateMintsParams`

---

## Related Documentation

- **`architecture.md`** - Overall `#[derive(LightAccounts)]` architecture and code generation
- **`pda.md`** - Compressed PDAs
- **`token.md`** - Token accounts (PDA-owned vaults)
- **`associated_token.md`** - Associated token accounts
- **`../light_program/`** - Program-level `#[light_program]` macro

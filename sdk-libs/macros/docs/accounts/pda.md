# Compressed PDA Creation

## Overview

Compressed PDAs are created using `#[light_account(init)]` on Anchor `Account<'info, T>`, `Box<Account<'info, T>>`, or `AccountLoader<'info, T>` fields. Tree info (address_tree_info, output_tree) is automatically fetched from `CreateAccountsProof` in the instruction parameters - no additional arguments are needed.

## Keywords

| Keyword | Description |
|---------|-------------|
| `init` | Required. Indicates account initialization for compression |
| `zero_copy` | Optional. Required for `AccountLoader<T>` fields using Pod serialization |

## Supported Field Types

| Type | Description |
|------|-------------|
| `Account<'info, T>` | Standard Anchor account |
| `Box<Account<'info, T>>` | Boxed account (for large accounts) |
| `AccountLoader<'info, T>` | Zero-copy account (requires `zero_copy` keyword) |

## Validation Rules

1. **`init` is required** - The `init` keyword must be the first argument
2. **`zero_copy` required for `AccountLoader`** - AccountLoader fields must include the `zero_copy` keyword
3. **`zero_copy` forbidden for non-`AccountLoader`** - Only AccountLoader fields can use `zero_copy`
4. **No namespace parameters allowed** - Tree info is auto-fetched from `CreateAccountsProof`; any `pda::` namespace parameters will cause a compile error

## Infrastructure Requirements

Infrastructure fields are auto-detected by naming convention. No attribute required.

| Field Type | Accepted Names |
|------------|----------------|
| Fee Payer | `fee_payer`, `payer`, `creator` |
| Compression Config | `compression_config` |
| PDA Rent Sponsor | `pda_rent_sponsor`, `compression_rent_sponsor` |

## Examples

### Standard PDA

```rust
#[derive(Accounts, LightAccounts)]
#[instruction(params: CreateParams)]
pub struct CreatePda<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + UserRecord::INIT_SPACE,
        seeds = [b"user", params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub user_record: Account<'info, UserRecord>,

    pub system_program: Program<'info, System>,
}
```

### Boxed Account

For large accounts that exceed stack limits:

```rust
#[account(
    init,
    payer = fee_payer,
    space = 8 + LargeRecord::INIT_SPACE,
    seeds = [b"large", params.id.as_ref()],
    bump,
)]
#[light_account(init)]
pub large_record: Box<Account<'info, LargeRecord>>,
```

### Zero-Copy PDA

For performance-critical accounts with fixed layouts using Pod serialization:

```rust
#[account(
    init,
    payer = fee_payer,
    space = 8 + core::mem::size_of::<ZcRecord>(),
    seeds = [b"zc_record", params.owner.as_ref()],
    bump,
)]
#[light_account(init, zero_copy)]
pub zc_record: AccountLoader<'info, ZcRecord>,
```

**Requirements for zero-copy accounts:**
- Data type must implement `bytemuck::Pod` and `bytemuck::Zeroable`
- Uses direct memory mapping instead of Borsh deserialization
- Incompatible with standard Borsh decompression path

## How Tree Info is Resolved

The macro automatically sources tree info from `CreateAccountsProof`:

- `address_tree_info` -> `params.create_accounts_proof.address_tree_info`
- `output_tree` -> `params.create_accounts_proof.output_state_tree_index`

If the proof is passed as a direct instruction argument (not nested in `params`), the macro detects this and adjusts the path accordingly.

## Generated Code

For each PDA field, the macro generates:

1. **Account extraction** - Gets account info and key
2. **Address tree extraction** - Resolves address tree pubkey from CPI accounts
3. **CompressionInfo initialization** - Sets compression info from config
4. **Address registration** - Calls `prepare_compressed_account_on_init`
5. **Rent reimbursement** - Transfers rent from sponsor PDA to fee payer

## Source References

- `sdk-libs/macros/src/light_pdas/accounts/pda.rs` - PDA block code generation
- `sdk-libs/macros/src/light_pdas/accounts/light_account.rs` - Attribute parsing (PdaField struct)
- `sdk-libs/macros/src/light_pdas/accounts/parse.rs` - Infrastructure field detection

## Related Documentation

- `architecture.md` - Overall LightAccounts derive macro architecture
- `mint.md` - Compressed mints
- `token.md` - Token accounts
- `associated_token.md` - Associated token accounts

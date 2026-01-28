# light-token-client

Rust client for light-token. Each action is a struct with an
async `execute` method that builds, signs, and sends the transaction.

## Actions

| Struct | Fields | `execute` signers |
|--------|--------|-------------------|
| `CreateMint` | `decimals`, `freeze_authority?`, `token_metadata?`, `seed?` | `payer`, `mint_authority` |
| `CreateAta` | `mint`, `owner`, `idempotent` | `payer` |
| `MintTo` | `mint`, `destination`, `amount` | `payer`, `authority` |
| `Transfer` | `source`, `destination`, `amount` | `payer`, `authority` |
| `TransferChecked` | `source`, `mint`, `destination`, `amount`, `decimals` | `payer`, `authority` |
| `TransferInterface` | `source`, `mint`, `destination`, `amount`, `decimals`, `spl_token_program?`, `restricted` | `payer`, `authority` |
| `Approve` | `token_account`, `delegate`, `amount`, `owner?` | `payer` (or `payer` + `owner`) |
| `Revoke` | `token_account`, `owner?` | `payer` (or `payer` + `owner`) |
| `Wrap` | `source_spl_ata`, `destination`, `mint`, `amount`, `decimals` | `payer`, `authority` |
| `Unwrap` | `source`, `destination_spl_ata`, `mint`, `amount`, `decimals` | `payer`, `authority` |

`?` = `Option`. All structs derive `Default`, `Clone`, `Debug`.

`CreateMint::execute` requires `R: Rpc + Indexer` (needs address proof).
All others require `R: Rpc`.

`Approve` and `Revoke` also expose `execute_with_owner` for when
owner differs from payer.

## Re-exports

From `light_token::instruction`:
- `derive_associated_token_account`
- `get_associated_token_address`
- `get_associated_token_address_and_bump`

## Supporting types

```rust
pub struct TokenMetadata {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub update_authority: Option<Pubkey>,
    pub additional_metadata: Option<Vec<(String, String)>>,
}
```

## Source layout

```
src/
  lib.rs              -- re-exports actions::*
  actions/
    mod.rs            -- submodule declarations, re-exports
    create_mint.rs    -- CreateMint, TokenMetadata
    create_ata.rs     -- CreateAta
    mint_to.rs        -- MintTo
    transfer.rs       -- Transfer
    transfer_checked.rs -- TransferChecked
    transfer_interface.rs -- TransferInterface
    approve.rs        -- Approve
    revoke.rs         -- Revoke
    wrap.rs           -- Wrap
    unwrap.rs         -- Unwrap
```
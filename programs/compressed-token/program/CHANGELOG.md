# Changelog

All notable changes to this package will be documented in this file.

## 2026-03-10

### Breaking Changes

- `CreateMintInputs` now requires a `rent_sponsor` field. Mint creation charges `MINT_CREATION_FEE` (50,000 lamports), transferred from the fee payer to the `rent_sponsor`. (#2309)
  Before: `CreateMintInputs { ... }` -- no rent_sponsor field.
  After: `CreateMintInputs { ..., rent_sponsor: Pubkey }` -- use `MintActionMetaConfig::with_rent_sponsor()` to configure the recipient.
  Migration: Add a `rent_sponsor` account to all mint creation calls and set it via `MintActionMetaConfig::with_rent_sponsor()`.

- `handle_compressible_top_up()` and `process_compressible_top_up()` now take a `FEE_PAYER_IDX` const generic. `APPROVE_PAYER_IDX` and `REVOKE_PAYER_IDX` are renamed to `OWNER_IDX`. The optional `FEE_PAYER_IDX` specifies a fee payer with fallback to the owner. (#2306)
  Before: `handle_compressible_top_up::<BASE_LEN, OWNER_IDX>(...)`
  After: `handle_compressible_top_up::<BASE_LEN, OWNER_IDX, FEE_PAYER_IDX>(...)`
  Migration: Update const generic parameters and rename `APPROVE_PAYER_IDX`/`REVOKE_PAYER_IDX` to `OWNER_IDX` in all call sites.

### Fixes

- Additional self-transfer validation prevents invalid self-transfers. (#2292)
- `create_token_account()` now checks rent exemption before creating the account. (#2292)
- `create_ata_idempotent()` now guards against double-creation. (#2292)

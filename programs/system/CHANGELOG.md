# Changelog

All notable changes to this package will be documented in this file.

## 2026-03-10

### Breaking Changes

- `COMPRESSIBLE_CONFIG_SEED` renamed to `LIGHT_CONFIG_SEED`. (#2222)
  Before: `COMPRESSIBLE_CONFIG_SEED`
  After: `LIGHT_CONFIG_SEED`
  Migration: Update all references to `COMPRESSIBLE_CONFIG_SEED`.

- `COMPRESSIBLE_CONFIG_V1` renamed to `LIGHT_TOKEN_CONFIG`. (#2222)
  Before: `COMPRESSIBLE_CONFIG_V1`
  After: `LIGHT_TOKEN_CONFIG`
  Migration: Update all references to `COMPRESSIBLE_CONFIG_V1`.

- In `#[light_account]`, `token::authority` renamed to `token::owner_seeds`. Owner seeds must now be constants. (#2222)
  Before: `#[light_account(token::authority = ...)]`
  After: `#[light_account(token::owner_seeds = ...)]`
  Migration: Rename `token::authority` to `token::owner_seeds` and ensure all values are constants.

- `#[light_account(init)]` now requires a `pda_rent_sponsor` account info. (#2222)
  Before: No `pda_rent_sponsor` required.
  After: `pda_rent_sponsor` account must be present in instruction accounts when initializing compressed PDAs.
  Migration: Add `pda_rent_sponsor` to all instruction accounts that use `#[light_account(init)]`.

- `#[derive(Compressible)]` removed from `light-sdk-macros`. (#2230)
  Before: `#[derive(Compressible)]`
  After: Use `#[derive(LightAccount)]` (Anchor/Solana) or `#[derive(LightProgramPinocchio)]` (Pinocchio).
  Migration: Replace all `#[derive(Compressible)]` usages with the appropriate new derive macro.

### Features

- New `light-account` crate provides Anchor/Solana-specific type aliases (`CpiAccounts`, `CompressCtx`, `DecompressCtx`, `ValidatedPdaContext`, `PackedAccounts`) and re-exports all macros from `light-sdk-macros`. (#2230)
- New `light-account-pinocchio` crate provides Pinocchio-specific type aliases and re-exports `#[derive(LightProgramPinocchio)]`. (#2230)
- `AccountLoader` added for loading compressed accounts without derive macros. (#2222)
- `DECOMPRESSED_PDA_DISCRIMINATOR` constant (`[255u8; 8]`) added to `light-compressible` to mark decompressed PDA placeholder accounts. (#2208)
- Compressed mint photon API added. (#2198)
- V1 tree initialization now logs a deprecation warning. V1 trees will be removed in a future release. (#2329)

### Fixes

- `MintCloseAuthority` added to `RESTRICTED_EXTENSION_TYPES` and `has_mint_extensions()` detection. A mint with this extension could previously be compressed without `CompressOnly` mode, allowing the mint authority to close the mint and strand compressed tokens. Certora audit finding M-03. (#2263)
- `store_data()` no longer caches the incorrect owner when re-entering a cached account context. (#2277)
- V2 tree rollover balance check corrected. (#2278)
- Canonical bump is now enforced during ATA verification. (#2249)
- Batched address tree initialization now asserts tree index and queue index match. (#2318)
- System program addresses corrected. (#2298)

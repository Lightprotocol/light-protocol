# Changelog

All notable changes to this package will be documented in this file.

## 2026-03-10

### Breaking Changes

- `create_initialize_merkle_tree_instruction()`, `create_initialize_batched_merkle_tree_instruction()`, and `create_initialize_batched_address_merkle_tree_instruction()` now require the protocol authority as signer. The `payer` parameter is renamed to `authority` and must be `protocol_config_pda.authority`.
  Before: `create_initialize_merkle_tree_instruction(payer, ...)`
  After: `create_initialize_merkle_tree_instruction(authority, ...)` -- `authority` must be the protocol config authority.
  Migration: Replace the payer account with the protocol authority in all tree initialization calls. (#2325)

### Features

- V1 tree initialization now logs a deprecation warning. V1 trees will be removed in a future release. (#2329)

### Fixes

- `init_v1_tree_with_custom_forester()` now correctly sets the custom forester on v1 trees. (#2319)
- `migrate_trees_ix()` no longer discards in-progress work during migration. (#2320)

## 2026-03-02

### Features

- `is_registration_phase()` no longer enforces the registration time window check. Foresters can now register for an epoch at any time within the activation window, not only during the designated registration phase. (#2321)

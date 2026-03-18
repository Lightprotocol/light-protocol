# Changelog

All notable changes to this package will be documented in this file.

## 2026-03-18

### Features

- `batch_append()` reimburses the forester fee payer 2x the network fee from the output queue when network_fee >= 5,000 lamports. A `fee_payer` account is now required in the instruction. (#2335)
- `batch_update_address_tree()` reimburses the forester fee payer 1x the network fee from the merkle tree when network_fee >= 5,000 lamports. A `fee_payer` account is now required in the instruction. (#2335)
- `nullify_dedup` instruction batches 2-4 nullifications in one transaction using proof deduplication.
- `nullify_2` uses a shared proof node and a 1-byte discriminator.
- Forester dedup integration adds `min_queue_items` threshold, versioned transaction support, and a transaction size fix.
- V1 state multi-nullify is disabled when the queue exceeds 10,000 items.

### Fixes

- `count_from_leaf_indices()` rejects non-trailing sentinels.

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

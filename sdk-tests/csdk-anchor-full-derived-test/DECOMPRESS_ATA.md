# Decompress ATAs via CPI

## Overview

Decompress compressed ATAs (compression_only) to on-chain CToken accounts. **Multiple ATAs can batch into ONE CPI** using `decompress_full_ctoken_accounts_with_indices`.

## Key Design: Explicit Indices

Each `CompressedAtaAccountData` specifies its own indices into `packed_accounts`:

```rust
CompressedAtaAccountData {
    meta: ...,
    data: ...,
    wallet_index: u8,  // index into packed_accounts
    mint_index: u8,    // index into packed_accounts
    ata_index: u8,     // index into packed_accounts
}
```

This allows **arbitrary de-duplication**:

- Shared mint: multiple ATAs use same `mint_index`
- Shared wallet: multiple ATAs use same `wallet_index`
- Unique everything: each ATA gets distinct indices

## Account Layout

```
remaining_accounts:
[0] ctoken_program - REQUIRED for invoke()
[1-5] system accounts (light_system, cpi_auth, registered, compression_auth, compression_prog)
[6+] packed_accounts (arbitrary order, referenced by indices)
```

On-chain code accesses accounts via:

```rust
let wallet = &packed_accounts[ata_account_data.wallet_index as usize];
let mint = &packed_accounts[ata_account_data.mint_index as usize];
let ata = &packed_accounts[ata_account_data.ata_index as usize];
```

## Client: Use PackedAccounts for De-duplication

```rust
use light_sdk::instruction::PackedAccounts;
let mut packed = PackedAccounts::default();

// Trees first
let state_tree_idx = packed.insert_or_get(tree);           // writable
let input_queue_idx = packed.insert_or_get(queue);         // writable
let _output_queue_idx = packed.insert_or_get(output_queue); // writable

// For each ATA
for (ata_pubkey, mint_pubkey, wallet) in atas {
    let wallet_idx = packed.insert_or_get_config(wallet.pubkey(), true, false); // signer, read-only
    let mint_idx = packed.insert_or_get_read_only(mint_pubkey);                 // read-only
    let ata_idx = packed.insert_or_get(ata_pubkey);                             // writable
    // Use these indices in CompressedAtaAccountData
}

// Get de-duplicated AccountMetas
let (packed_account_metas, _, _) = packed.to_account_metas();
remaining_accounts.extend(packed_account_metas);
```

## Footguns

1. **Index reference frame** - Indices are relative to `packed_accounts` (0-based from position 6 in remaining_accounts). NOT the full remaining_accounts array.

2. **Duplicate pubkeys rejected** - ctoken program rejects duplicate pubkeys in accounts list. **De-duplicate on client** and use same index for repeated pubkeys.

3. **compression_index uniqueness** - Must be unique per input in the batch. Use `i as u8` where `i` is the ATA's index in params.compressed_accounts.

4. **Wallet is signer, ATA is not** - ATA is a PDA, can't sign. Wallet must sign. Mark wallet as `is_signer: true` in AccountMeta.

5. **ATA owner = ATA pubkey** - For compression_only ATAs, the compressed account's `owner` field = ATA address (PDA). Fetch via `get_compressed_token_accounts_by_owner(&ata_pubkey, ...)`.

6. **Bounds check** - On-chain code validates `max(wallet_index, mint_index, ata_index) < packed_accounts.len()`. Ensure indices are valid.

7. **Tree indices consistency** - `merkle_tree_pubkey_index` and `queue_pubkey_index` in `meta.tree_info` must match the actual positions of the tree accounts in packed_accounts.

8. **Rent sponsor** - Use `CTOKEN_RENT_SPONSOR` from SDK. Protocol's rent sponsor is whitelisted; custom sponsors require additional signature logic.

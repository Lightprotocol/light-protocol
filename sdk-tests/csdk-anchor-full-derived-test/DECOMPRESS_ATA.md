# Decompress ATAs via CPI

## Overview

Decompress compressed ATAs (compression_only) to on-chain CToken accounts. **Multiple ATAs can batch into ONE CPI** using `decompress_full_ctoken_accounts_with_indices`.

## Data Format: PACKED (indices only)

Uses same pattern as ctoken's `MultiTokenTransferOutputData` - **client sends indices, on-chain unpacks**.

```rust
// ~14 bytes per ATA (vs ~77-109 unpacked)
pub struct PackedAtaTokenData {
    pub wallet_index: u8,      // index into packed_accounts
    pub mint_index: u8,        // index into packed_accounts
    pub ata_index: u8,         // index into packed_accounts
    pub amount: u64,           // actual value
    pub has_delegate: bool,    // flag
    pub delegate_index: u8,    // index (0 if none)
    pub is_frozen: bool,       // actual value
}
```

On-chain unpacks indices to pubkeys:

```rust
let wallet_pubkey = packed_accounts[packed_data.wallet_index].key;
let mint_pubkey = packed_accounts[packed_data.mint_index].key;
// Derive and validate ATA
let (expected_ata, bump) = derive_ctoken_ata(wallet_pubkey, mint_pubkey);
```

## Account Layout

```
remaining_accounts:
[0] ctoken_program - REQUIRED for invoke()
[1-5] system accounts
[6+] packed_accounts (arbitrary order, referenced by indices)
```

## Client: Use PackedAccounts

```rust
use light_sdk::instruction::PackedAccounts;
let mut packed = PackedAccounts::default();

// Trees first
let state_tree_idx = packed.insert_or_get(tree);
let input_queue_idx = packed.insert_or_get(queue);
let _output_queue_idx = packed.insert_or_get(output_queue);

// For each ATA - de-duplication automatic!
let wallet_idx = packed.insert_or_get_config(wallet.pubkey(), true, false);
let mint_idx = packed.insert_or_get_read_only(mint_pubkey);
let ata_idx = packed.insert_or_get(ata_pubkey);

// Build PACKED params
PackedAtaAccountData {
    meta: ...,
    data: PackedAtaVariant::Standard(PackedAtaTokenData {
        wallet_index: wallet_idx,
        mint_index: mint_idx,
        ata_index: ata_idx,
        amount,
        has_delegate: false,
        delegate_index: 0,
        is_frozen: false,
    }),
}

// Get de-duplicated AccountMetas
let (packed_account_metas, _, _) = packed.to_account_metas();
remaining_accounts.extend(packed_account_metas);
```

## Footguns

1. **Duplicate pubkeys rejected** - ctoken program rejects duplicate pubkeys. `PackedAccounts::insert_or_get` handles de-duplication automatically.

2. **compression_index uniqueness** - Must be unique per input. Use `i as u8` (loop index).

3. **Wallet signs, ATA doesn't** - ATA is PDA. Mark wallet as `is_signer: true` via `insert_or_get_config(pubkey, true, false)`.

4. **ATA owner = ATA pubkey** - For compression_only ATAs, compressed account's `owner` = ATA address. Fetch via `get_compressed_token_accounts_by_owner(&ata_pubkey, ...)`.

5. **On-chain unpacks** - Indices are resolved via `packed_accounts[index].key`. Validate ATA derivation matches.

6. **Rent sponsor** - Use `CTOKEN_RENT_SPONSOR` from SDK (whitelisted).

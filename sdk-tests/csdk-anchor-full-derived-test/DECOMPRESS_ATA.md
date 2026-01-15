# Decompress ATAs

## TL;DR

- Multiple ATAs batch into **ONE CPI** via `decompress_full_ctoken_accounts_with_indices`
- Packed format: **~14 bytes/ATA** (vs ~77 unpacked)
- Client sends indices, on-chain unpacks to pubkeys

---

## Packed Format

```rust
pub struct PackedAtaTokenData {
    pub wallet_index: u8,      // index into packed_accounts
    pub mint_index: u8,        // index into packed_accounts
    pub ata_index: u8,         // index into packed_accounts
    pub amount: u64,           // raw value
    pub has_delegate: bool,
    pub delegate_index: u8,    // 0 if none
    pub is_frozen: bool,
}
```

---

## Footguns

### 1. Duplicate Pubkeys Rejected

ctoken program rejects duplicate pubkeys. **Always use `PackedAccounts`**:

```rust
let mint_idx = packed.insert_or_get_read_only(mint_pubkey); // reused automatically
```

### 2. compression_index Uniqueness

Must be unique per ATA. Use loop index:

```rust
compression_index: i as u8,
```

### 3. Wallet Signs, Not ATA

ATA is a PDA. Mark wallet as signer:

```rust
packed.insert_or_get_config(wallet.pubkey(), true, false); // signer=true
```

### 4. ATA Owner = ATA Pubkey

For `compression_only` ATAs, compressed account's `owner` = ATA address.

```rust
rpc.get_compressed_token_accounts_by_owner(&ata_pubkey, ...) // NOT wallet
```

### 5. `is_ata: true` Flag

Required in `CompressedOnlyExtensionInstructionData`. Tells ctoken that wallet signs.

### 6. Rent Sponsor

Use `CTOKEN_RENT_SPONSOR` from SDK.

### 7. Index Reference Frame

Indices are relative to `packed_accounts` slice (after system accounts).

---

## Client Pattern

```rust
let mut packed = PackedAccounts::default();

// Trees
let _tree_idx = packed.insert_or_get(state_tree);
let _queue_idx = packed.insert_or_get(queue);
let _output_idx = packed.insert_or_get(output_queue);

// Per ATA (de-duplication automatic)
let wallet_idx = packed.insert_or_get_config(wallet.pubkey(), true, false);
let mint_idx = packed.insert_or_get_read_only(mint_pubkey);
let ata_idx = packed.insert_or_get(ata_pubkey);

// Build packed params
PackedAtaTokenData {
    wallet_index: wallet_idx,
    mint_index: mint_idx,
    ata_index: ata_idx,
    amount,
    has_delegate: false,
    delegate_index: 0,
    is_frozen: false,
}
```

## On-chain Unpacking

```rust
let wallet = &packed_accounts[packed_data.wallet_index];
let mint = &packed_accounts[packed_data.mint_index];
let (expected_ata, bump) = derive_ctoken_ata(wallet.key, mint.key);
```

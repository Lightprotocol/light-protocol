# Unified Decompress (ATAs + CMint)

**Status: ✅ IMPLEMENTED**

## TL;DR

- **Any number of ATAs** + **at most 1 CMint** per instruction
- Single `Vec<DecompressUnifiedAccountData>` with `DecompressVariant::Ata` or `DecompressVariant::Mint`
- Packed format: ~14 bytes/ATA, ~50 bytes/Mint (vs ~77/~180 unpacked)
- CPI context used when mixing types (mint writes first, ATAs execute)

## Files

- `src/instruction_accounts.rs`: types
- `src/lib.rs`: `decompress_unified` handler
- `tests/basic_test.rs`: `test_decompress_unified_structure`

---

## Critical Footguns

### 1. At Most 1 Mint (On-Chain Enforced)

Returns `ConstraintRaw` if >1 mint variant passed. Client should validate before sending.

### 2. Indices Reference Frame

All indices in `PackedAtaTokenData` / `PackedMintTokenData` are relative to `packed_accounts` slice:

```rust
// On-chain
let packed_accounts = &remaining[offset + 6..]; // AFTER system accounts
let wallet = &packed_accounts[packed_data.wallet_index];
```

### 3. CPI Context Position

When mixing mint + ATAs, CPI context **must** be at `remaining[offset + 6]`:

```
[0-5] system accounts
[6]   cpi_context (only if mint + atas)
[7+]  packed_accounts
```

### 4. Signers

| Account       | Must Sign?                                |
| ------------- | ----------------------------------------- |
| `fee_payer`   | ✅ Always                                 |
| `authority`   | ✅ If decompressing mint (mint authority) |
| Wallet owners | ✅ Each wallet for ATAs                   |
| ATA PDAs      | ❌ Never                                  |

### 5. De-Duplication Required

Client **must** use `PackedAccounts` to de-duplicate. ctoken program rejects duplicate pubkeys:

```rust
let mut packed = PackedAccounts::default();
let mint_idx = packed.insert_or_get_read_only(mint_pubkey); // reused for all ATAs with same mint
```

### 6. ATA Owner = ATA Pubkey

For `compression_only` ATAs:

- Compressed account's `owner` = ATA address (PDA)
- Fetch via `get_compressed_token_accounts_by_owner(&ata_pubkey, ...)`
- `is_ata: true` flag tells ctoken that wallet signs, not the ATA PDA

### 7. compressed_address is DATA, Not Account

CMint's `compressed_address: [u8; 32]` is Light protocol address for proof verification.
**Do NOT add to packed_accounts.** It's raw instruction data.

### 8. compression_index Uniqueness

Must be unique per input ATA. Use loop index:

```rust
compression_index: i as u8, // i from enumerate()
```

### 9. Rent Sponsor

Always use `CTOKEN_RENT_SPONSOR` from SDK. Custom rent sponsors require whitelisting.

### 10. CPI Account Ordering

`invoke()` expects exact order from instruction builder. **Don't include ctoken_program in account_infos** - it's already in `instruction.program_id`.

### 11. Writability

```rust
packed.insert_or_get(pubkey)         // writable (cmint_pda, ata, trees)
packed.insert_or_get_read_only(...)  // read-only (mint_seed, mint for ata)
packed.insert_or_get_config(p, signer, writable) // custom
```

### 12. output_state_tree_index

Stored in `meta.output_state_tree_index`. Don't assume position relative to input_queue.

### 13. Pubkey Type Conversion

Light uses its own `Pubkey`. Use `.into()` when constructing CPI data:

```rust
mint_authority: mint_authority.map(|p| p.into()),
metadata: CompressedMintMetadata { mint: cmint_pda.into(), ... }
```

---

## Execution Flow

```
ATAs only:
  decompress_full_ctoken_accounts_with_indices(proof, None, ...)

Mint only:
  DecompressCMint::instruction() + invoke()

Mint + ATAs:
  1. DecompressCMintCpiWithContext(first_set_context=true) → writes to context
  2. decompress_full_ctoken_accounts_with_indices(proof, Some(cpi_context)) → executes
```

---

## Account Layout

```
remaining_accounts:
[0]  ctoken_program              (read-only)
[1]  light_system_program        (read-only)
[2]  cpi_authority               (read-only)
[3]  registered_program          (read-only)
[4]  acc_compression_authority   (read-only)
[5]  acc_compression_program     (read-only)
[6]  cpi_context                 (writable, ONLY if mint+atas)
[7+] trees, output_queue, mints, wallets, atas, cmint_pda, mint_seed...
```

---

## Packed Data Structures

### ATA (~14 bytes)

```rust
pub struct PackedAtaTokenData {
    pub wallet_index: u8,      // index
    pub mint_index: u8,        // index
    pub ata_index: u8,         // index
    pub amount: u64,           // raw value
    pub has_delegate: bool,
    pub delegate_index: u8,    // 0 if none
    pub is_frozen: bool,
}
```

### Mint (~50 bytes + extensions)

```rust
pub struct PackedMintTokenData {
    pub mint_seed_index: u8,            // index
    pub cmint_pda_index: u8,            // index
    pub compressed_address: [u8; 32],   // RAW DATA (not index!)
    pub leaf_index: u32,
    pub prove_by_index: bool,
    pub root_index: u16,
    pub supply: u64,
    pub decimals: u8,
    pub version: u8,
    pub cmint_decompressed: bool,
    pub has_mint_authority: bool,
    pub mint_authority_index: u8,       // index (0 if none)
    pub has_freeze_authority: bool,
    pub freeze_authority_index: u8,     // index (0 if none)
    pub rent_payment: u8,
    pub write_top_up: u32,
    pub extensions: Option<Vec<ExtensionInstructionData>>,
}
```

---

## Size Comparison

| Scenario               | Unpacked   | Packed     | Savings |
| ---------------------- | ---------- | ---------- | ------- |
| 1 ATA                  | ~77 bytes  | ~14 bytes  | 82%     |
| 1 Mint (no metadata)   | ~150 bytes | ~50 bytes  | 67%     |
| 1 Mint (with metadata) | ~380 bytes | ~250 bytes | 34%     |
| 5 ATAs + 1 Mint        | ~565 bytes | ~120 bytes | 79%     |

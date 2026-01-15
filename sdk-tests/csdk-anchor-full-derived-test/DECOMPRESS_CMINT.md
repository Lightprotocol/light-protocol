# Decompress CMint

## TL;DR

- **One CPI per mint** - `DecompressMint` does NOT support CPI context batching
- Packed format: **~50 bytes/Mint** (vs ~150-180 unpacked, metadata extra)
- `compressed_address` is **RAW DATA**, not an account index

---

## Packed Format

```rust
pub struct PackedMintTokenData {
    pub mint_seed_index: u8,            // INDEX - Solana account
    pub cmint_pda_index: u8,            // INDEX - Solana account
    pub compressed_address: [u8; 32],   // RAW DATA - Light address
    pub leaf_index: u32,
    pub prove_by_index: bool,
    pub root_index: u16,
    pub supply: u64,
    pub decimals: u8,
    pub version: u8,
    pub cmint_decompressed: bool,
    pub has_mint_authority: bool,
    pub mint_authority_index: u8,       // INDEX (0 if none)
    pub has_freeze_authority: bool,
    pub freeze_authority_index: u8,     // INDEX (0 if none)
    pub rent_payment: u8,
    pub write_top_up: u32,
    pub extensions: Option<Vec<ExtensionInstructionData>>,
}
```

### What's Packed vs Raw

| Field                | Treatment                         |
| -------------------- | --------------------------------- |
| `mint_seed_pubkey`   | **Index** (Solana account)        |
| `cmint_pda`          | **Index** (Solana account)        |
| `mint_authority`     | **Index** (Solana account)        |
| `freeze_authority`   | **Index** (Solana account)        |
| `compressed_address` | **Raw [u8;32]** - NOT an account! |
| `extensions`         | **Raw** - variable metadata       |

---

## Footguns

### 1. compressed_address is DATA

Light protocol address used in proof verification. **Do NOT add to packed_accounts**.

```rust
compressed_address: cmint_compressed_address, // raw [u8;32], NOT an index
```

### 2. CPI Account Order

`invoke()` expects exact order from instruction builder. **Don't include ctoken_program in account_infos**:

```rust
let account_infos = vec![
    light_system_program, mint_seed, authority, compressible_config,
    cmint, rent_sponsor, fee_payer, cpi_authority, registered_program,
    acc_compression_authority, acc_compression_program, system_program,
    output_queue, state_tree, input_queue,
];
// NO ctoken_program - it's from instruction.program_id
```

### 3. Writability Matters

```rust
packed.insert_or_get(cmint_pda);          // writable
packed.insert_or_get_read_only(mint_seed); // read-only
```

### 4. output_state_tree_index

Stored in `meta.output_state_tree_index`. Not derived from input_queue position.

### 5. Pubkey Type Conversion

Light uses its own `Pubkey`. Convert with `.into()`:

```rust
mint_authority: mint_authority.map(|p| p.into()),
metadata: CompressedMintMetadata { mint: cmint_pda.into(), ... }
```

### 6. Authority Must Sign

`authority` field is the mint authority that was set during mint creation. Must sign.

---

## Size Comparison

| Config           | Unpacked   | Packed     | Savings |
| ---------------- | ---------- | ---------- | ------- |
| No authorities   | ~150 bytes | ~50 bytes  | **67%** |
| With authorities | ~180 bytes | ~52 bytes  | **71%** |
| + 200 char URI   | ~380 bytes | ~250 bytes | **34%** |

---

## Client Pattern

```rust
let mut packed = PackedAccounts::default();

// System accounts [0-5]
packed.insert_or_get_read_only(C_TOKEN_PROGRAM_ID);
// ... etc

// Trees from validity proof
let packed_tree_infos = proof_result.pack_tree_infos(&mut packed);
let output_queue_idx = packed.insert_or_get(output_queue_pubkey);

// Mint accounts
let mint_seed_idx = packed.insert_or_get_read_only(mint_signer.pubkey());
let cmint_pda_idx = packed.insert_or_get(cmint_pda);
let mint_authority_idx = packed.insert_or_get_read_only(authority.pubkey());

PackedMintTokenData {
    mint_seed_index: mint_seed_idx,
    cmint_pda_index: cmint_pda_idx,
    compressed_address: cmint_compressed_address, // RAW, not index!
    has_mint_authority: true,
    mint_authority_index: mint_authority_idx,
    // ...
}
```

## On-chain Unpacking

```rust
let mint_seed = &remaining[offset + packed.mint_seed_index as usize];
let cmint = &remaining[offset + packed.cmint_pda_index as usize];

// Raw data, no unpacking
let compressed_address: [u8; 32] = packed.compressed_address;

// Optional authority
let mint_authority = if packed.has_mint_authority {
    Some(*remaining[offset + packed.mint_authority_index as usize].key)
} else {
    None
};
```

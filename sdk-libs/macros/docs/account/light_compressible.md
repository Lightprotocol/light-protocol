# LightCompressible Derive Macro

## 1. Overview

The `#[derive(LightCompressible)]` macro is a convenience derive that combines all traits required for a fully compressible account. It is the recommended way to prepare account structs for Light Protocol's rent-free compression system.

**When to use**: Apply this derive to any account struct that will be used with `#[rentfree]` in an Accounts struct. This is the standard approach for most use cases.

**Source**: `sdk-libs/macros/src/rentfree/traits/light_compressible.rs` (lines 56-79)

---

## 2. How It Works

### 2.1 Compile-Time Expansion

```
#[derive(LightCompressible)]
              |
              v
+----------------------------------+
|   derive_rentfree_account()      |
|   (light_compressible.rs:56)     |
+----------------------------------+
              |
              +---> derive_light_hasher_sha()
              |           |
              |           v
              |     DataHasher + ToByteArray impls
              |
              +---> discriminator()
              |           |
              |           v
              |     LightDiscriminator impl
              |
              +---> derive_compressible()
              |           |
              |           v
              |     HasCompressionInfo + CompressAs +
              |     Size + CompressedInitSpace impls
              |
              +---> derive_compressible_pack()
                          |
                          v
                    Pack + Unpack impls +
                    Packed{Name} struct
```

### 2.2 Full Transformation Flow

```
INPUT                           GENERATED
+---------------------------+   +------------------------------------------+
| #[derive(LightCompressible)]  | // 8+ trait implementations               |
| pub struct UserRecord {   |   |                                          |
|   pub owner: Pubkey,      |   | impl DataHasher for UserRecord { ... }   |
|   pub score: u64,         |   | impl ToByteArray for UserRecord { ... }  |
|   pub compression_info:   |   | impl LightDiscriminator for UserRecord { |
|     Option<CompressionInfo>   |   const LIGHT_DISCRIMINATOR = [...];     |
| }                         |   | }                                        |
+---------------------------+   | impl HasCompressionInfo for UserRecord { |
                                |   fn compression_info() -> &...          |
                                |   fn compression_info_mut() -> &mut ...  |
                                | }                                        |
                                | impl CompressAs for UserRecord { ... }   |
                                | impl Size for UserRecord { ... }         |
                                | impl CompressedInitSpace for UserRecord {|
                                | impl Pack for UserRecord { ... }         |
                                | impl Unpack for UserRecord { ... }       |
                                | pub struct PackedUserRecord { ... }      |
                                | impl Pack for PackedUserRecord { ... }   |
                                | impl Unpack for PackedUserRecord { ... } |
                                +------------------------------------------+
```

### 2.3 Role in Compression Lifecycle

```
                          COMPRESSION LIFECYCLE
                          ====================

+-------------------+     +-------------------+     +-------------------+
|   Data Struct     | --> |   Accounts Struct | --> |   Runtime         |
+-------------------+     +-------------------+     +-------------------+
| #[derive(         |     | #[derive(Accounts,|     | light_pre_init()  |
|  LightCompressible)]   |   RentFree)]       |     |   Uses:           |
|                   |     | #[instruction]    |     |   - DataHasher    |
| Provides:         |     | pub struct Create |     |   - LightDiscrim. |
| - Hashing         |     | {                 |     |   - HasCompression|
| - Discriminator   |     |   #[rentfree]     |     |     Info          |
| - Compression     |     |   pub user_record |     |   - CompressAs    |
| - Pack/Unpack     |     | }                 |     |   - Size          |
+-------------------+     +-------------------+     |   - Pack          |
                                                    +-------------------+
```

---

## 3. Generated Traits

`LightCompressible` expands to four derive macros:

| Derive | Traits Generated |
|--------|------------------|
| `LightHasherSha` | `DataHasher`, `ToByteArray` |
| `LightDiscriminator` | `LightDiscriminator` |
| `Compressible` | `HasCompressionInfo`, `CompressAs`, `Size`, `CompressedInitSpace` |
| `CompressiblePack` | `Pack`, `Unpack`, `Packed{Name}` struct |

### Equivalent Manual Derives

```rust
// This:
#[derive(LightCompressible)]
pub struct MyAccount { ... }

// Is equivalent to:
#[derive(LightHasherSha, LightDiscriminator, Compressible, CompressiblePack)]
pub struct MyAccount { ... }
```

---

## 4. Required Field

The struct **must** have a field named `compression_info` of type `Option<CompressionInfo>`:

```rust
pub struct MyAccount {
    pub data: u64,
    pub compression_info: Option<CompressionInfo>,  // Required
}
```

---

## 5. Supported Attributes

### `#[compress_as(field = expr, ...)]` - Field Overrides

Override specific field values in the compressed representation (passed to `Compressible` derive):

```rust
#[derive(LightCompressible)]
#[compress_as(start_time = 0, cached_value = 0)]
pub struct GameSession {
    pub session_id: u64,
    pub player: Pubkey,
    pub start_time: u64,      // Will be 0 in compressed form
    pub cached_value: u64,    // Will be 0 in compressed form
    pub compression_info: Option<CompressionInfo>,
}
```

### `#[skip]` - Exclude Fields

Mark fields to exclude from compression and size calculations:

```rust
#[derive(LightCompressible)]
pub struct CachedData {
    pub id: u64,
    #[skip]  // Excluded from compression
    pub cached_timestamp: u64,
    pub compression_info: Option<CompressionInfo>,
}
```

---

## 6. Complete Code Example

### Input

```rust
use anchor_lang::prelude::*;
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::LightCompressible;

#[derive(Default, Debug, Clone, InitSpace, LightCompressible)]
#[account]
pub struct UserRecord {
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub score: u64,
    pub compression_info: Option<CompressionInfo>,
}
```

### Generated Output Summary

```rust
// From LightHasherSha:
impl light_hasher::DataHasher for UserRecord { ... }
impl light_hasher::ToByteArray for UserRecord { ... }

// From LightDiscriminator:
impl light_sdk::discriminator::LightDiscriminator for UserRecord {
    const LIGHT_DISCRIMINATOR: &'static [u8] = &[...];  // 8-byte unique ID
}

// From Compressible:
impl light_sdk::compressible::HasCompressionInfo for UserRecord { ... }
impl light_sdk::compressible::CompressAs for UserRecord { ... }
impl light_sdk::account::Size for UserRecord { ... }
impl light_sdk::compressible::CompressedInitSpace for UserRecord { ... }

// From CompressiblePack:
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedUserRecord {
    pub owner: u8,  // Pubkey compressed to index
    pub name: String,
    pub score: u64,
    pub compression_info: Option<CompressionInfo>,
}
impl light_sdk::compressible::Pack for UserRecord { ... }
impl light_sdk::compressible::Unpack for UserRecord { ... }
impl light_sdk::compressible::Pack for PackedUserRecord { ... }
impl light_sdk::compressible::Unpack for PackedUserRecord { ... }
```

---

## 7. Hashing Behavior

The `LightHasherSha` component uses SHA256 to hash the entire struct:

- **No `#[hash]` attributes needed** - SHA256 serializes and hashes all fields
- **Type 3 ShaFlat hashing** - Efficient flat serialization for hashing
- The `compression_info` field is included in the serialized form but typically set to `None`

---

## 8. Discriminator

The `LightDiscriminator` component generates an 8-byte unique identifier:

```rust
const LIGHT_DISCRIMINATOR: &'static [u8] = &[0x12, 0x34, ...];  // SHA256("light:UserRecord")[..8]
```

This discriminator is used to identify account types in compressed account data.

---

## 9. Pubkey Packing

If the struct contains `Pubkey` fields, `CompressiblePack` generates:

- A `Packed{Name}` struct with `Pubkey` fields replaced by `u8` indices
- `Pack` implementation to convert to packed form
- `Unpack` implementation to restore from packed form

If no `Pubkey` fields exist, identity implementations are generated instead.

---

## 10. Usage with RentFree

`LightCompressible` prepares the data struct for use with `#[derive(RentFree)]` on Accounts structs:

```rust
// Data struct - apply LightCompressible
#[derive(Default, Debug, Clone, InitSpace, LightCompressible)]
#[account]
pub struct UserRecord {
    pub owner: Pubkey,
    pub score: u64,
    pub compression_info: Option<CompressionInfo>,
}

// Accounts struct - apply RentFree
#[derive(Accounts, RentFree)]
#[instruction(params: CreateParams)]
pub struct Create<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    #[account(init, payer = fee_payer, space = 8 + UserRecord::INIT_SPACE, ...)]
    #[rentfree]
    pub user_record: Account<'info, UserRecord>,
}
```

---

## 11. Usage Notes

- The struct must derive `Clone` (required by `CompressiblePack`)
- The struct should derive Anchor's `InitSpace` (required by `CompressedInitSpace`)
- The `compression_info` field is auto-detected and handled specially (no `#[skip]` needed)
- Only works with named-field structs, not tuple structs or unit structs
- Enums are not supported

---

## 12. Error Conditions

| Error | Cause |
|-------|-------|
| `LightCompressible can only be derived for structs` | Applied to enum or union |
| `Struct must have a 'compression_info' field` | Missing required field |

---

## 13. Related Macros

| Macro | Relationship |
|-------|--------------|
| [`HasCompressionInfo`](has_compression_info.md) | Included via `Compressible` |
| [`CompressAs`](compress_as.md) | Included via `Compressible` |
| [`Compressible`](compressible.md) | Included in `LightCompressible` |
| [`CompressiblePack`](compressible_pack.md) | Included in `LightCompressible` |
| [`RentFree`](../rentfree.md) | Uses traits from `LightCompressible` |

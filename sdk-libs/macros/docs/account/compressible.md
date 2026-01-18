# Compressible Derive Macro

## 1. Overview

The `#[derive(Compressible)]` macro is a combined derive that generates all core compression traits needed for an account struct. It is the recommended way to add compression support when you don't need hashing or discriminator traits.

**When to use**: Apply this derive when you need compression traits but are handling hashing and discriminator separately. For full compression support, use `#[derive(LightCompressible)]` instead.

**Source**: `sdk-libs/macros/src/rentfree/traits/traits.rs` (lines 233-272)

---

## 2. How It Works

### 2.1 Compile-Time Expansion

```
+------------------+     +--------------------+     +--------------------+
|  Input Struct    | --> |   Compressible     | --> |   4 Trait Impls    |
|                  |     |   Macro            |     |                    |
+------------------+     +--------------------+     +--------------------+
| #[derive(        |     | Expands to 4       |     | - HasCompression-  |
|  Compressible)]  |     | internal derives:  |     |   Info             |
| pub struct User {|     |                    |     | - CompressAs       |
|   owner: Pubkey, |     | 1. HasCompression- |     | - Size             |
|   score: u64,    |     |    Info            |     | - CompressedInit-  |
|   compression_   |     | 2. CompressAs      |     |   Space            |
|   info: ...      |     | 3. Size            |     |                    |
| }                |     | 4. CompressedInit- |     |                    |
|                  |     |    Space           |     |                    |
+------------------+     +--------------------+     +--------------------+
```

### 2.2 Trait Generation Pipeline

```
derive_compressible()
        |
        +---> validate_compression_info_field()
        |          |
        |          v
        |     Error if missing compression_info field
        |
        +---> generate_has_compression_info_impl()
        |          |
        |          v
        |     HasCompressionInfo trait impl
        |
        +---> generate_compress_as_field_assignments()
        |          |
        |          +---> Process each field
        |          |        - Skip compression_info
        |          |        - Skip #[skip] fields
        |          |        - Apply #[compress_as] overrides
        |          |        - Copy vs Clone detection
        |          v
        |     generate_compress_as_impl()
        |
        +---> generate_size_fields()
        |          |
        |          v
        |     Size trait impl
        |
        +---> generate_compressed_init_space_impl()
                   |
                   v
              CompressedInitSpace trait impl
```

### 2.3 Role in Compression System

The four traits work together during compression/decompression:

```
COMPRESSION FLOW
+------------------------+
| Account Data           |
+------------------------+
         |
         | HasCompressionInfo
         v
+------------------------+
| Set compression_info   |
| with address, lamports |
+------------------------+
         |
         | CompressAs
         v
+------------------------+
| Create clean copy for  |
| hashing (no metadata)  |
+------------------------+
         |
         | Size
         v
+------------------------+
| Calculate byte size    |
| for Merkle tree leaf   |
+------------------------+
         |
         | CompressedInitSpace
         v
+------------------------+
| Verify fits in 800     |
| byte limit             |
+------------------------+
```

---

## 3. Generated Traits

The `Compressible` derive generates implementations for four traits:

| Trait | Purpose |
|-------|---------|
| `HasCompressionInfo` | Accessor methods for `compression_info` field |
| `CompressAs` | Creates compressed representation for hashing |
| `Size` | Calculates serialized byte size |
| `CompressedInitSpace` | Provides `COMPRESSED_INIT_SPACE` constant |

### Equivalent Manual Derives

```rust
// This:
#[derive(Compressible)]
pub struct MyAccount { ... }

// Is equivalent to:
#[derive(HasCompressionInfo, CompressAs, Size)]  // + CompressedInitSpace
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

Override specific field values in the compressed representation:

```rust
#[derive(Compressible)]
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

Mark fields to exclude from both `CompressAs` output and `Size` calculation:

```rust
#[derive(Compressible)]
pub struct CachedData {
    pub id: u64,
    #[skip]  // Excluded from compression and size
    pub cached_timestamp: u64,
    pub compression_info: Option<CompressionInfo>,
}
```

---

## 6. Generated Code Example

### Input

```rust
use anchor_lang::prelude::*;
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::Compressible;

#[derive(Clone, InitSpace, Compressible)]
#[compress_as(cached_score = 0)]
pub struct UserRecord {
    pub owner: Pubkey,
    pub score: u64,
    pub cached_score: u64,
    pub compression_info: Option<CompressionInfo>,
}
```

### Generated Output

```rust
// HasCompressionInfo implementation
impl light_sdk::compressible::HasCompressionInfo for UserRecord {
    fn compression_info(&self) -> &light_sdk::compressible::CompressionInfo {
        self.compression_info.as_ref().expect("compression_info must be set")
    }

    fn compression_info_mut(&mut self) -> &mut light_sdk::compressible::CompressionInfo {
        self.compression_info.as_mut().expect("compression_info must be set")
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<light_sdk::compressible::CompressionInfo> {
        &mut self.compression_info
    }

    fn set_compression_info_none(&mut self) {
        self.compression_info = None;
    }
}

// CompressAs implementation
impl light_sdk::compressible::CompressAs for UserRecord {
    type Output = Self;

    fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
        std::borrow::Cow::Owned(Self {
            compression_info: None,
            owner: self.owner,
            score: self.score,
            cached_score: 0,  // Override applied
        })
    }
}

// Size implementation
impl light_sdk::account::Size for UserRecord {
    fn size(&self) -> usize {
        // CompressionInfo space: 1 (Option discriminant) + INIT_SPACE
        let compression_info_size = 1 + <light_sdk::compressible::CompressionInfo
            as light_sdk::compressible::Space>::INIT_SPACE;
        compression_info_size
            + self.owner.try_to_vec().expect("Failed to serialize").len()
            + self.score.try_to_vec().expect("Failed to serialize").len()
            + self.cached_score.try_to_vec().expect("Failed to serialize").len()
    }
}

// CompressedInitSpace implementation
impl light_sdk::compressible::CompressedInitSpace for UserRecord {
    const COMPRESSED_INIT_SPACE: usize = Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE;
}
```

---

## 7. Size Calculation

The `Size` trait calculates the serialized byte size of the account:

- **CompressionInfo space**: Always allocates space for `Some(CompressionInfo)` since it will be set during decompression
- **Field serialization**: Uses `try_to_vec()` (Borsh serialization) for accurate size
- **Auto-skipped fields**: `compression_info` and `#[skip]` fields are excluded

---

## 8. CompressedInitSpace Calculation

The `CompressedInitSpace` trait provides:

```rust
const COMPRESSED_INIT_SPACE: usize = Self::LIGHT_DISCRIMINATOR.len() + Self::INIT_SPACE;
```

This requires the struct to also derive `LightDiscriminator` and Anchor's `InitSpace`.

---

## 9. Usage Notes

- The struct must derive `Clone` if it has non-Copy fields
- The struct should derive Anchor's `InitSpace` for `COMPRESSED_INIT_SPACE` to work
- For full compression support including hashing, use `#[derive(LightCompressible)]`

---

## 10. Related Macros

| Macro | Relationship |
|-------|--------------|
| [`HasCompressionInfo`](has_compression_info.md) | Included in `Compressible` |
| [`CompressAs`](compress_as.md) | Included in `Compressible` |
| [`CompressiblePack`](compressible_pack.md) | Pack/Unpack for Pubkey compression (separate derive) |
| [`LightCompressible`](light_compressible.md) | Includes `Compressible` + hashing + discriminator + pack |

# CompressiblePack Derive Macro

## 1. Overview

The `#[derive(CompressiblePack)]` macro generates `Pack` and `Unpack` trait implementations along with a `Packed{StructName}` struct. This enables efficient Pubkey compression where 32-byte Pubkeys are replaced with u8 indices into a remaining accounts array.

**When to use**: Apply this derive when you need to pack account data for compressed account instructions. This is automatically included in `#[derive(LightCompressible)]`.

**Source**: `sdk-libs/macros/src/rentfree/traits/pack_unpack.rs` (lines 8-186)

---

## 2. How It Works

### 2.1 Compile-Time Decision

```
derive_compressible_pack()
          |
          v
+-------------------------+
| Scan struct fields for  |
| Pubkey types            |
+-------------------------+
          |
    +-----+-----+
    |           |
    v           v
+-------+   +---------+
| Has   |   | No      |
| Pubkey|   | Pubkey  |
+-------+   +---------+
    |           |
    v           v
+---------------+   +------------------+
| Generate full |   | Generate type    |
| Packed struct |   | alias + identity |
| + conversions |   | impls            |
+---------------+   +------------------+
```

### 2.2 Pubkey Compression Flow

32-byte Pubkeys are compressed to 1-byte indices:

```
PACK (Client-side)
+---------------------------+        +---------------------------+
| UserRecord                |        | PackedUserRecord          |
+---------------------------+        +---------------------------+
| owner: ABC123...          |   ->   | owner: 0                  |
| authority: DEF456...      |   ->   | authority: 1              |
| score: 100                |   ->   | score: 100                |
+---------------------------+        +---------------------------+
                                              |
                                              v
                                     +------------------+
                                     | remaining_accounts|
                                     +------------------+
                                     | [0] ABC123...    |
                                     | [1] DEF456...    |
                                     +------------------+

UNPACK (On-chain)
+---------------------------+        +---------------------------+
| PackedUserRecord          |        | UserRecord                |
+---------------------------+        +---------------------------+
| owner: 0                  |   ->   | owner: ABC123...          |
| authority: 1              |   ->   | authority: DEF456...      |
| score: 100                |   ->   | score: 100                |
+---------------------------+        +---------------------------+
         ^
         |
+------------------+
| remaining_accounts|
| [0] = ABC123...  |
| [1] = DEF456...  |
+------------------+
```

### 2.3 Why Pack Pubkeys?

Compressed account instructions are serialized and stored in Merkle trees. Packing provides:

| Aspect | Unpacked | Packed | Savings |
|--------|----------|--------|---------|
| Single Pubkey | 32 bytes | 1 byte | 31 bytes |
| Two Pubkeys | 64 bytes | 2 bytes | 62 bytes |

The remaining accounts array stores actual Pubkeys, while instruction data contains only indices.

---

## 3. Generated Items

The macro generates different outputs based on whether the struct contains Pubkey fields:

### With Pubkey Fields

| Item | Type | Description |
|------|------|-------------|
| `Packed{StructName}` | Struct | New struct with Pubkeys replaced by `u8` |
| `Pack for StructName` | Trait impl | Converts struct to packed form |
| `Unpack for StructName` | Trait impl | Identity unpack (returns clone) |
| `Pack for Packed{StructName}` | Trait impl | Identity pack (returns clone) |
| `Unpack for Packed{StructName}` | Trait impl | Converts packed form back to original |

### Without Pubkey Fields

| Item | Type | Description |
|------|------|-------------|
| `Packed{StructName}` | Type alias | `type Packed{StructName} = {StructName}` |
| `Pack for StructName` | Trait impl | Identity pack (returns clone) |
| `Unpack for StructName` | Trait impl | Identity unpack (returns clone) |

---

## 4. Trait Signatures

### Pack Trait

```rust
pub trait Pack {
    type Packed;

    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed;
}
```

### Unpack Trait

```rust
pub trait Unpack {
    type Unpacked;

    fn unpack(
        &self,
        remaining_accounts: &[AccountInfo],
    ) -> Result<Self::Unpacked, ProgramError>;
}
```

---

## 5. Code Example - With Pubkey Fields

### Input

```rust
use anchor_lang::prelude::*;
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::CompressiblePack;

#[derive(Clone, CompressiblePack)]
pub struct UserRecord {
    pub owner: Pubkey,
    pub authority: Pubkey,
    pub score: u64,
    pub compression_info: Option<CompressionInfo>,
}
```

### Generated Output

```rust
// Packed struct with Pubkeys replaced by u8 indices
#[derive(Debug, Clone, anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
pub struct PackedUserRecord {
    pub owner: u8,           // Pubkey -> u8 index
    pub authority: u8,       // Pubkey -> u8 index
    pub score: u64,          // Non-Pubkey unchanged
    pub compression_info: Option<CompressionInfo>,
}

// Pack original -> packed
impl light_sdk::compressible::Pack for UserRecord {
    type Packed = PackedUserRecord;

    #[inline(never)]
    fn pack(&self, remaining_accounts: &mut light_sdk::instruction::PackedAccounts) -> Self::Packed {
        PackedUserRecord {
            owner: remaining_accounts.insert_or_get(self.owner),
            authority: remaining_accounts.insert_or_get(self.authority),
            score: self.score,
            compression_info: None,
        }
    }
}

// Unpack original -> original (identity)
impl light_sdk::compressible::Unpack for UserRecord {
    type Unpacked = Self;

    #[inline(never)]
    fn unpack(
        &self,
        _remaining_accounts: &[anchor_lang::prelude::AccountInfo],
    ) -> std::result::Result<Self::Unpacked, anchor_lang::prelude::ProgramError> {
        Ok(self.clone())
    }
}

// Pack packed -> packed (identity)
impl light_sdk::compressible::Pack for PackedUserRecord {
    type Packed = Self;

    #[inline(never)]
    fn pack(&self, _remaining_accounts: &mut light_sdk::instruction::PackedAccounts) -> Self::Packed {
        self.clone()
    }
}

// Unpack packed -> original
impl light_sdk::compressible::Unpack for PackedUserRecord {
    type Unpacked = UserRecord;

    #[inline(never)]
    fn unpack(
        &self,
        remaining_accounts: &[anchor_lang::prelude::AccountInfo],
    ) -> std::result::Result<Self::Unpacked, anchor_lang::prelude::ProgramError> {
        Ok(UserRecord {
            owner: *remaining_accounts[self.owner as usize].key,
            authority: *remaining_accounts[self.authority as usize].key,
            score: self.score,
            compression_info: None,
        })
    }
}
```

---

## 6. Code Example - Without Pubkey Fields

### Input

```rust
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::CompressiblePack;

#[derive(Clone, CompressiblePack)]
pub struct SimpleRecord {
    pub id: u64,
    pub value: u32,
    pub compression_info: Option<CompressionInfo>,
}
```

### Generated Output

```rust
// Type alias instead of new struct
pub type PackedSimpleRecord = SimpleRecord;

// Identity pack
impl light_sdk::compressible::Pack for SimpleRecord {
    type Packed = SimpleRecord;

    #[inline(never)]
    fn pack(&self, _remaining_accounts: &mut light_sdk::instruction::PackedAccounts) -> Self::Packed {
        self.clone()
    }
}

// Identity unpack
impl light_sdk::compressible::Unpack for SimpleRecord {
    type Unpacked = Self;

    #[inline(never)]
    fn unpack(
        &self,
        _remaining_accounts: &[anchor_lang::prelude::AccountInfo],
    ) -> std::result::Result<Self::Unpacked, anchor_lang::prelude::ProgramError> {
        Ok(self.clone())
    }
}
```

---

## 7. Field Handling

| Field Type | Pack Behavior | Unpack Behavior |
|------------|---------------|-----------------|
| `Pubkey` | `remaining_accounts.insert_or_get(pubkey)` -> `u8` | `*remaining_accounts[idx].key` -> `Pubkey` |
| `compression_info` | Always set to `None` | Always set to `None` |
| Copy types (`u64`, etc.) | Direct copy | Direct copy |
| Clone types (`String`, etc.) | `.clone()` | `.clone()` |

### Pubkey Type Detection

The macro recognizes these as Pubkey types:
- `Pubkey`
- `solana_pubkey::Pubkey`
- `anchor_lang::prelude::Pubkey`
- Other paths ending in `Pubkey`

---

## 8. Usage in Instructions

The pack/unpack system is used when building compressed account instructions:

```rust
// Client-side: pack account data
let mut packed_accounts = PackedAccounts::new();
let packed_record = user_record.pack(&mut packed_accounts);

// On-chain: unpack from instruction data
let user_record = packed_record.unpack(ctx.remaining_accounts)?;
```

---

## 9. Usage Notes

- The struct must implement `Clone`
- `compression_info` field is always set to `None` during pack/unpack
- All methods are marked `#[inline(never)]` for smaller program size
- The packed struct derives `AnchorSerialize` and `AnchorDeserialize`

### Limitation: Option<Pubkey> Fields

Only direct `Pubkey` fields are converted to `u8` indices. `Option<Pubkey>` fields remain as `Option<Pubkey>` in the packed struct because `None` doesn't map cleanly to an index.

```rust
pub struct Record {
    pub owner: Pubkey,           // -> u8 in packed struct
    pub delegate: Option<Pubkey>, // -> Option<Pubkey> in packed struct (unchanged)
}
```

---

## 10. Related Macros

| Macro | Relationship |
|-------|--------------|
| [`Compressible`](compressible.md) | Provides compression traits (separate concern) |
| [`LightCompressible`](light_compressible.md) | Includes `CompressiblePack` + all other traits |
| [`HasCompressionInfo`](has_compression_info.md) | Provides compression info accessors |

# LightAccount Derive Macro

## Overview

The `#[derive(LightAccount)]` macro is a unified derive that generates all required trait implementations for compressible account data structs. It handles the transformation of on-chain PDA state to compressed form in Merkle trees and back.

**Module Location:** `sdk-libs/macros/src/light_pdas/account/`

**Purpose:**
- Generate SHA256 hashing implementations for Merkle tree inclusion (via `LightHasherSha`)
- Generate discriminators for account type identification (via `LightDiscriminator`)
- Generate pack/unpack logic for Pubkey compression (32 bytes -> 1 byte index)
- Generate unified `LightAccount` trait implementation with compression_info accessors
- Enforce 800-byte size limit at compile time

**Note:** This is a unified macro that replaces the need for separate `#[derive(LightHasherSha, LightDiscriminator)]` - it generates all required traits in one derive.

---

## Quick Start

```rust
use light_sdk_macros::LightAccount;
use light_sdk::compressible::CompressionInfo;
use solana_pubkey::Pubkey;

#[derive(Default, Debug, Clone, InitSpace, LightAccount)]
#[account]
pub struct UserRecord {
    pub compression_info: CompressionInfo,  // Non-Option, first or last field
    pub owner: Pubkey,
    #[max_len(32)]
    pub name: String,
    pub score: u64,
}
```

---

## Account Data Lifecycle

```
                    ACCOUNT DATA LIFECYCLE
                    ======================

+------------------+                    +------------------+
|   Uncompressed   |                    |    Compressed    |
|   (On-chain PDA) |                    |  (Merkle Tree)   |
+------------------+                    +------------------+
        |                                       |
        | compress_and_close()                  | decompress_idempotent()
        | (via light_finalize)                  | (via light_pre_init)
        v                                       v
+------------------+                    +------------------+
| 1. Pack Pubkeys  |                    | 1. Unpack indices|
|    to u8 indices |                    |    to Pubkeys    |
| 2. Hash via SHA  |                    | 2. Verify hash   |
| 3. Set comp_info |                    | 3. Restore PDA   |
|    to Compressed |                    |    with data     |
+------------------+                    +------------------+
        |                                       |
        v                                       v
+------------------+                    +------------------+
| Write to output  |                    | Account ready    |
| state tree       |                    | for modification |
+------------------+                    +------------------+
```

---

## Generated Items

The `LightAccount` derive generates all required traits and supporting types:

| Generated Item | Type | Purpose |
|----------------|------|---------|
| `impl DataHasher for T` | Trait impl | SHA256-based hashing for Merkle tree inclusion |
| `impl ToByteArray for T` | Trait impl | Serialize struct using Borsh for hashing |
| `impl LightDiscriminator for T` | Trait impl | 8-byte discriminator from struct name SHA256 |
| `impl LightAccount for T` | Trait impl | Unified trait with pack/unpack, compression_info accessors |
| `PackedT` struct | Struct | Pubkeys replaced with u8 indices, compression_info excluded |
| `impl AnchorSerialize/Deserialize for T` | Trait impl (zero-copy only) | Required for `#[account(zero_copy)]` Pod types |
| Size assertion | Compile-time check | Ensures INIT_SPACE <= 800 bytes |
| **V1 Compatibility Traits** | | |
| `impl Pack for T` | Trait impl (client-only) | V1 compatibility - delegates to LightAccount::pack |
| `impl Unpack for PackedT` | Trait impl | V1 compatibility - delegates to LightAccount::unpack |
| `impl HasCompressionInfo for T` | Trait impl | V1 compatibility - wraps non-Option compression_info |
| `impl Size for T` | Trait impl | V1 compatibility - uses INIT_SPACE |
| `impl CompressAs for T` | Trait impl | V1 compatibility - creates compressed clone |
| `impl CompressedInitSpace for T` | Trait impl | V1 compatibility - space calculation |

---

## compression_info Field Requirements

```rust
#[derive(LightAccount)]
pub struct UserRecord {
    pub compression_info: CompressionInfo,  // REQUIRED: non-Option type
    pub owner: Pubkey,
    pub score: u64,
}
```

**Requirements:**
- Field must be named `compression_info`
- Type must be `CompressionInfo` (NOT `Option<CompressionInfo>`)
- Must be **first or last** field in the struct
- Excluded from `PackedT` struct (saves space in compressed form)

**Why non-Option?**
- V2 accounts use non-Option `CompressionInfo` (simpler, clearer semantics)
- V1 compatibility traits wrap this as `Option<CompressionInfo>` when needed
- `CompressionInfo::compressed()` represents the "None" state for V1 compatibility

**Why first or last?**
- Enables efficient serialization/deserialization at known offsets
- Allows direct byte-slice manipulation without full deserialization
- Optimizes decompression by writing only compression_info bytes

---

## Pack/Unpack Mechanism

### Overview

The macro generates a `PackedXxx` struct where:
- `Pubkey` fields become `u8` indices (32 bytes -> 1 byte)
- `compression_info` is excluded entirely
- Other fields are preserved

This reduces on-chain storage costs while maintaining full account semantics.

### Packing (Client-Side)

**Input struct:**
```rust
pub struct UserRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,      // 32 bytes
    pub authority: Pubkey,  // 32 bytes
    pub score: u64,
}
```

**Generated packed struct:**
```rust
#[derive(Debug, Clone, anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
pub struct PackedUserRecord {
    // compression_info EXCLUDED entirely
    pub owner: u8,          // 1 byte (index into accounts array)
    pub authority: u8,      // 1 byte
    pub score: u64,
}
```

**Generated pack method:**
```rust
impl light_sdk::interface::LightAccount for UserRecord {
    type Packed = PackedUserRecord;

    fn pack(
        &self,
        accounts: &mut light_sdk::instruction::PackedAccounts,
    ) -> Result<Self::Packed, ProgramError> {
        Ok(PackedUserRecord {
            owner: accounts.insert_or_get_read_only(self.owner),
            authority: accounts.insert_or_get_read_only(self.authority),
            score: self.score,
        })
    }
}
```

**For Copy types:** `self.field` is used directly
**For non-Copy types:** `self.field.clone()` is used

### Unpacking (On-Chain)

Indices are resolved back to Pubkeys using the `remaining_accounts` array:

**Generated unpack method:**
```rust
impl light_sdk::interface::LightAccount for UserRecord {
    fn unpack<A: light_sdk::light_account_checks::AccountInfoTrait>(
        packed: &Self::Packed,
        accounts: &light_sdk::light_account_checks::packed_accounts::ProgramPackedAccounts<A>,
    ) -> Result<Self, ProgramError> {
        Ok(UserRecord {
            compression_info: light_sdk::compressible::CompressionInfo::compressed(),
            owner: {
                let account = accounts
                    .get_u8(packed.owner, "UserRecord: owner")
                    .map_err(|_| ProgramError::InvalidAccountData)?;
                solana_pubkey::Pubkey::from(account.key())
            },
            authority: {
                let account = accounts
                    .get_u8(packed.authority, "UserRecord: authority")
                    .map_err(|_| ProgramError::InvalidAccountData)?;
                solana_pubkey::Pubkey::from(account.key())
            },
            score: packed.score,
        })
    }
}
```

**Error messages:** Include struct name and field name for debugging (e.g., `"UserRecord: owner"`)

### Special Cases

**No Pubkey fields:**
If the struct has no Pubkey fields (only primitives), the packed struct still excludes `compression_info` but preserves all other fields as-is.

**Only compression_info:**
If the struct only has a `compression_info` field:
```rust
#[derive(Debug, Clone, anchor_lang::AnchorSerialize, anchor_lang::AnchorDeserialize)]
pub struct PackedMinimal;
```

---

## Hashing Strategy (SHA256)

The `LightAccount` macro generates `LightHasherSha` which uses SHA256-based hashing:

1. **Serialize entire struct** using Borsh (`try_to_vec()`)
2. **Hash serialized bytes** with SHA256
3. **Truncate first byte to 0** (ensures < 254 bits for BN254 field)

**Generated code:**
```rust
// Generated by LightAccount derive (via LightHasherSha)
impl light_hasher::DataHasher for UserRecord {
    fn hash<H>(&self) -> Result<[u8; 32], light_hasher::errors::HasherError>
    where H: light_hasher::Hasher
    {
        let serialized = self.try_to_vec()
            .map_err(|_| light_hasher::errors::HasherError::BorshError)?;
        let mut result = H::hash(&serialized)?;
        result[0] = 0;  // Truncate to field size
        Ok(result)
    }
}

impl light_hasher::to_byte_array::ToByteArray for UserRecord {
    fn to_byte_array(&self) -> Result<Vec<Vec<u8>>, light_hasher::errors::HasherError> {
        let serialized = self.try_to_vec()
            .map_err(|_| light_hasher::errors::HasherError::BorshError)?;
        Ok(vec![serialized])
    }
}
```

**Note:** The entire struct is serialized (including all fields). No `#[hash]` or `#[skip]` attributes are needed for basic usage.

---

## Size Constraints

### Maximum Compressed Account Size: 800 bytes

The `LightAccount` derive enforces a compile-time size assertion:

```rust
// For Borsh-serialized types (default)
const _: () = {
    assert!(
        <UserRecord as anchor_lang::Space>::INIT_SPACE <= 800,
        "Compressed account size exceeds 800 byte limit"
    );
};

// For zero-copy (Pod) types
const _: () = {
    assert!(
        core::mem::size_of::<ZeroCopyRecord>() <= 800,
        "Compressed account size exceeds 800 byte limit"
    );
};
```

**Why 800 bytes?**
- ZK proof circuits have fixed input sizes
- 800 bytes is the maximum data payload for compressed account leaves
- Larger accounts require splitting or alternative storage strategies

**What counts toward the limit?**
- For normal accounts: `<T as anchor_lang::Space>::INIT_SPACE` (from `#[derive(InitSpace)]`)
- For zero-copy accounts: `core::mem::size_of::<T>()`
- The `compression_info` field is included in this calculation

**If you exceed the limit:**
```
error: Compressed account size exceeds 800 byte limit
  --> src/state.rs:10:1
   |
10 | #[derive(LightAccount)]
   | ^^^^^^^^^^^^^^^^^^^^^^^
```

**Solutions:**
1. Reduce field sizes or counts
2. Use smaller types (e.g., `u16` instead of `u64`)
3. Split data across multiple accounts
4. Use references/indices to off-chain data

---

## Zero-Copy Support

For `#[account(zero_copy)]` structs, the macro generates additional implementations:

```rust
#[derive(LightAccount)]
#[account(zero_copy)]
#[repr(C)]
pub struct ZeroCopyRecord {
    pub compression_info: CompressionInfo,
    pub value: u64,
}
```

**Detection:**
The macro detects zero-copy mode by checking for `#[account(zero_copy)]` attribute (not just `#[repr(C)]`).

**Generated differences for zero-copy:**

1. **AnchorSerialize/AnchorDeserialize** - Field-by-field serialization using `anchor_lang::` paths
   ```rust
   impl anchor_lang::AnchorSerialize for ZeroCopyRecord {
       fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
           anchor_lang::AnchorSerialize::serialize(&self.compression_info, writer)?;
           anchor_lang::AnchorSerialize::serialize(&self.value, writer)?;
           Ok(())
       }
   }
   ```

2. **AccountType** - Uses `AccountType::PdaZeroCopy` instead of `AccountType::Pda`

3. **Size calculation** - Uses `core::mem::size_of::<Self>()` instead of Anchor's `INIT_SPACE`

4. **Size assertion** - Checks `core::mem::size_of::<T>()` instead of `<T as Space>::INIT_SPACE`

**Why field-by-field serialization?**
The workspace `borsh` dependency and `anchor_lang`'s internal borsh resolve to different crate instances (proc-macro boundary causes duplication). Using `#[derive(BorshSerialize)]` would generate impls for the wrong borsh instance. By generating field-by-field impls with fully-qualified `anchor_lang::` paths, we ensure compatibility.

---

## Attribute: `#[compress_as(field = value)]`

Override field values during decompression (reset transient fields):

```rust
#[derive(LightAccount)]
#[compress_as(start_time = 0, temp_data = [0u8; 32])]
pub struct GameSession {
    pub compression_info: CompressionInfo,
    pub game_id: u64,           // Kept as-is
    pub start_time: u64,        // Reset to 0 on decompress
    pub temp_data: [u8; 32],    // Reset to zeros on decompress
}
```

**Where it's used:**

1. **In `set_decompressed()`** - Called during decompression to initialize PDA
   ```rust
   fn set_decompressed(&mut self, config: &LightConfig, current_slot: u64) {
       self.compression_info = CompressionInfo::new_from_config(config, current_slot);
       self.start_time = 0;
       self.temp_data = [0u8; 32];
   }
   ```

2. **In `CompressAs::compress_as()`** - V1 compatibility trait
   ```rust
   fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
       let mut result = self.clone();
       result.compression_info = CompressionInfo::compressed();
       result.start_time = 0;
       result.temp_data = [0u8; 32];
       std::borrow::Cow::Owned(result)
   }
   ```

**Use cases:**
- Reset timestamps during decompression
- Clear temporary runtime state
- Initialize session-specific data
- Zero out transient fields

**Auto-skipped fields:**
- `compression_info` (automatically set by the macro)
- Fields marked with `#[skip]` attribute

---

## Discriminator Generation

The `LightAccount` derive generates `LightDiscriminator` which creates an 8-byte identifier:

**Generated code:**
```rust
// Generated by LightAccount derive (via LightDiscriminator)
impl light_discriminator::LightDiscriminator for UserRecord {
    const LIGHT_DISCRIMINATOR: [u8; 8] = [/* 8 bytes from SHA256("UserRecord") */];
}
```

**How it works:**
```rust
// Hash the struct name
let hash_input = "UserRecord".to_string();
let hash = Sha256::hash(hash_input.as_bytes());

// Take first 8 bytes
let discriminator = &hash[..8];
```

**Note:** The discriminator is independent of Anchor's discriminator (which uses `"account:StructName"` format). Light discriminators are used for identifying compressed account types in the Merkle tree.

---

## File Structure

```
sdk-libs/macros/src/light_pdas/account/
|-- mod.rs                # Module exports
|-- derive.rs             # LightAccount derive macro (MAIN FILE)
|                         #   - derive_light_account() - Entry point, orchestrates all code generation
|                         #   - generate_light_account_impl() - Generates unified LightAccount trait
|                         #   - generate_packed_struct() - Creates PackedXxx struct
|                         #   - generate_pack_body() - Generates pack() method
|                         #   - generate_unpack_body() - Generates unpack() method
|                         #   - generate_compress_as_assignments() - Handles #[compress_as(...)]
|                         #   - generate_anchor_serde_for_zero_copy() - For Pod types
|-- traits.rs             # Legacy V1 trait derives
|                         #   - derive_compress_as() - V1 CompressAs trait
|                         #   - derive_has_compression_info() - V1 HasCompressionInfo trait
|                         #   - parse_compress_as_overrides() - Parses #[compress_as(...)]
|-- validation.rs         # Account validation
|                         #   - validate_compression_info_field() - Ensures first/last position
|                         #   - AccountTypeError - Error types for validation
+-- utils.rs              # Shared utility functions
                          #   - extract_fields_from_derive_input() - Extract struct fields
                          #   - is_copy_type() - Detect Copy types for clone optimization
                          #   - is_pubkey_type() - Detect Pubkey fields for packing
```

**Related files:**
- `../discriminator.rs` - `discriminator()` function (generates LightDiscriminator impl)
- `../hasher/` - `derive_light_hasher_sha()` function (generates DataHasher + ToByteArray impls)

---

## V1 Compatibility Traits

The `LightAccount` derive automatically generates V1 compatibility traits for backward compatibility:

| Trait | Purpose | Implementation |
|-------|---------|---------------|
| `Pack` | Client-side packing | Delegates to `LightAccount::pack()` |
| `Unpack` | On-chain unpacking | Delegates to `LightAccount::unpack()` |
| `HasCompressionInfo` | Compression info access | Wraps non-Option as Option |
| `Size` | Space calculation | Returns `INIT_SPACE` |
| `CompressAs` | Clone with overrides | Applies `#[compress_as(...)]` fields |
| `CompressedInitSpace` | Space constant | `LIGHT_DISCRIMINATOR.len() + INIT_SPACE` |

**Why these exist:**
- Light Protocol V1 used `Option<CompressionInfo>` (V2 uses non-Option)
- V1 client code expects these trait impls
- Generated impls provide seamless migration path

**V2 code should use:**
- `LightAccount::pack()` instead of `Pack::pack()`
- `LightAccount::unpack()` instead of `Unpack::unpack()`
- `LightAccount::compression_info()` instead of `HasCompressionInfo::compression_info()`

## Related Documentation

- **`../accounts/architecture.md`** - `#[derive(LightAccounts)]` for Accounts structs
- **`../accounts/pda.md`** - `#[light_account(init)]` field attribute
- **`../light_program/`** - `#[light_program]` attribute macro
- **`../CLAUDE.md`** - Main macro documentation entry point

## Source Code Reference

**Main implementation:**
- `/Users/ananas/dev/light-protocol2/sdk-libs/macros/src/light_pdas/account/derive.rs`
  - `derive_light_account()` - Entry point
  - `generate_light_account_impl()` - Core trait generation
  - `generate_packed_struct()` - PackedXxx struct
  - `generate_pack_body()` / `generate_unpack_body()` - Pack/unpack logic

**Supporting modules:**
- `/Users/ananas/dev/light-protocol2/sdk-libs/macros/src/light_pdas/account/traits.rs` - V1 traits
- `/Users/ananas/dev/light-protocol2/sdk-libs/macros/src/light_pdas/account/validation.rs` - Validation
- `/Users/ananas/dev/light-protocol2/sdk-libs/macros/src/discriminator.rs` - Discriminator generation
- `/Users/ananas/dev/light-protocol2/sdk-libs/macros/src/hasher/` - Hasher generation

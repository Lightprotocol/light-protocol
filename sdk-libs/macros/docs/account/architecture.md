# LightAccount Derive Macro

## Overview

The `#[derive(LightAccount)]` macro generates trait implementations for compressible account data structs. It handles the transformation of on-chain PDA state to compressed form in Merkle trees and back.

**Module Location:** `sdk-libs/macros/src/light_pdas/account/`

**Purpose:**
- Generate hashing implementations for Merkle tree inclusion
- Generate discriminators for account type identification
- Generate pack/unpack logic for Pubkey compression (32 bytes -> 1 byte index)
- Generate unified `LightAccount` trait implementation

---

## Quick Start

```rust
use light_sdk_macros::LightAccount;
use light_sdk::compressible::CompressionInfo;
use solana_pubkey::Pubkey;

#[derive(Default, Debug, Clone, InitSpace, LightAccount)]
#[account]
pub struct UserRecord {
    pub compression_info: CompressionInfo,  // First or last field, non-Option
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

| Generated Item | Type | Purpose |
|----------------|------|---------|
| `impl DataHasher for T` | Trait impl | SHA256-based hashing for Merkle tree inclusion |
| `impl ToByteArray for T` | Trait impl | Serialize struct to 32-byte array for hashing |
| `impl LightDiscriminator for T` | Trait impl | 8-byte discriminator from struct name SHA256 |
| `impl LightAccount for T` | Trait impl | Unified trait with pack/unpack, compression_info accessors |
| `PackedT` struct | Struct | Pubkeys replaced with u8 indices, compression_info excluded |
| `impl Pack for T` | Trait impl (client-only) | Convert T to PackedT with index mapping |
| `impl Unpack for PackedT` | Trait impl | Convert PackedT back to T using account array |
| `impl CompressedInitSpace for T` | Trait impl | Compile-time space calculation |

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
- Type must be `CompressionInfo` (not `Option<CompressionInfo>`)
- Must be **first or last** field in the struct
- Excluded from `PackedT` struct (saves 24 bytes in compressed form)

**Why first or last?**
- Enables efficient `write_decompressed_info_to_slice()` without full deserialization
- Allows direct byte-slice manipulation at known offsets
- Optimizes decompression by writing only compression_info bytes

---

## Pack/Unpack Mechanism

### Packing (Client-Side)

Pubkeys are replaced with u8 indices into a shared `PackedAccounts` array:

```rust
// Input struct
pub struct UserRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,      // 32 bytes
    pub authority: Pubkey,  // 32 bytes
    pub score: u64,
}

// Generated packed struct
pub struct PackedUserRecord {
    // compression_info EXCLUDED (saves 24 bytes)
    pub owner: u8,          // 1 byte (index into accounts array)
    pub authority: u8,      // 1 byte
    pub score: u64,
}
```

### Unpacking (On-Chain)

Indices are resolved back to Pubkeys using the remaining_accounts array:

```rust
fn unpack<A: AccountInfoTrait>(
    packed: &PackedUserRecord,
    accounts: &ProgramPackedAccounts<A>,
) -> Result<UserRecord, ProgramError> {
    Ok(UserRecord {
        compression_info: CompressionInfo::compressed(),
        owner: Pubkey::from(accounts.get_u8(packed.owner, "UserRecord: owner")?.key()),
        authority: Pubkey::from(accounts.get_u8(packed.authority, "UserRecord: authority")?.key()),
        score: packed.score,
    })
}
```

---

## Hashing Strategy (SHA256)

The `LightAccount` macro uses SHA256-based hashing:

1. **Serialize entire struct** using Borsh (`try_to_vec()`)
2. **Hash serialized bytes** with SHA256
3. **Truncate first byte to 0** (ensures < 254 bits for BN254 field)

```rust
impl DataHasher for UserRecord {
    fn hash<H>(&self) -> Result<[u8; 32], HasherError>
    where H: Hasher
    {
        let serialized = self.try_to_vec().map_err(|_| HasherError::BorshError)?;
        let mut result = H::hash(&serialized)?;
        result[0] = 0;  // Truncate to field size
        Ok(result)
    }
}
```

---

## Size Constraints

### Maximum Compressed Account Size: 800 bytes

The `LightAccount` derive enforces a compile-time size assertion:

```rust
// For Borsh-serialized types (default)
const _: () = {
    assert!(
        <T as anchor_lang::Space>::INIT_SPACE <= 800,
        "Compressed account size exceeds 800 byte limit"
    );
};

// For zero-copy (Pod) types
const _: () = {
    assert!(
        core::mem::size_of::<T>() <= 800,
        "Compressed account size exceeds 800 byte limit"
    );
};
```

**Why 800 bytes?**
- ZK proof circuits have fixed input sizes
- 800 bytes is the maximum data payload for compressed account leaves
- Larger accounts require splitting or alternative storage strategies

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

**Generated:**
- `AnchorSerialize` / `AnchorDeserialize` implementations for Pod types
- `AccountType::PdaZeroCopy` constant
- Size calculation via `core::mem::size_of::<Self>()`

---

## Attribute: `#[compress_as(field = value)]`

Override field values during compression:

```rust
#[derive(LightAccount)]
#[compress_as(start_time = 0, temp_data = [0u8; 32])]
pub struct GameSession {
    pub compression_info: CompressionInfo,
    pub game_id: u64,           // Kept as-is
    pub start_time: u64,        // Reset to 0 on compress
    pub temp_data: [u8; 32],    // Reset to zeros on compress
}
```

Generated in `set_decompressed()`:
```rust
fn set_decompressed(&mut self, config: &LightConfig, current_slot: u64) {
    self.compression_info = CompressionInfo::new_from_config(config, current_slot);
    self.start_time = 0;
    self.temp_data = [0u8; 32];
}
```

---

## Discriminator Generation

The discriminator is an 8-byte identifier derived from the struct name:

```rust
// With anchor-discriminator feature
let hash_input = format!("account:{}", account_name);  // "account:UserRecord"

// Without anchor-discriminator feature
let hash_input = account_name.to_string();  // "UserRecord"

// First 8 bytes of SHA256 hash
let discriminator = &Sha256::hash(hash_input.as_bytes())[..8];
```

---

## File Structure

```
sdk-libs/macros/src/light_pdas/account/
|-- mod.rs                # Module exports
|-- light_compressible.rs # LightAccount derive macro implementation
|                         #   - derive_light_account()
|                         #   - generate_light_account_impl()
|                         #   - generate_packed_struct()
|                         #   - generate_pack_body() / generate_unpack_body()
|-- pack_unpack.rs        # Standalone Pack/Unpack generation
|                         #   - derive_compressible_pack()
|-- seed_extraction.rs    # Anchor seed extraction from #[account(seeds = [...])]
|                         #   - extract_anchor_seeds()
|                         #   - ClassifiedSeed enum
|-- traits.rs             # Standalone trait derives (used by LightAccount internally)
|                         #   - derive_compress_as()
|                         #   - derive_has_compression_info()
+-- utils.rs              # Shared utility functions
                          #   - extract_fields_from_derive_input()
                          #   - is_copy_type() / is_pubkey_type()
```

---

## Related Documentation

- **`../accounts/architecture.md`** - `#[derive(LightAccounts)]` for Accounts structs
- **`../accounts/pda.md`** - `#[light_account(init)]` field attribute
- **`../light_program/`** - `#[light_program]` attribute macro

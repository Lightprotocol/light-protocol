# LightAccount Derive Macro

## Overview

`#[derive(LightAccount)]` generates all traits required for a compressible account data struct.

This replaces the current `#[derive(LightCompressible)]` with a cleaner, unified trait.

---

## Account Types

```rust
enum AccountType {
    Pda,
    PdaZeroCopy,
    Token,
    Ata,
    Mint,
}
```

| Type | Description | Data Struct |
|------|-------------|-------------|
| `Pda` | Custom PDA with borsh serialization | User-defined (derives `LightAccount`) |
| `PdaZeroCopy` | Custom PDA with zero-copy serialization | User-defined (derives `LightAccount`) |
| `Token` | SPL Token account | `TokenData` (SDK pre-implemented) |
| `Ata` | Associated Token Account | `TokenData` (SDK pre-implemented) |
| `Mint` | SPL Mint account | `MintData` (SDK pre-implemented) |

---

## Trait Definition

```rust
trait LightAccount: Sized + Clone + AnchorSerialize + AnchorDeserialize {
    /// Packed version (Pubkeys -> u8 indices)
    type Packed: AnchorSerialize + AnchorDeserialize;

    /// 8-byte discriminator for compressed account identification
    const DISCRIMINATOR: [u8; 8];

    /// Compile-time size for space allocation
    const INIT_SPACE: usize;

    // --- Hashing ---

    /// Hash the account data for Merkle tree storage (SHA256)
    fn hash<H: Hasher>(&self) -> Result<[u8; 32], HasherError>;

    // --- Compression Info ---

    /// Get compression info reference
    fn compression_info(&self) -> &CompressionInfo;

    /// Get mutable compression info reference
    fn compression_info_mut(&mut self) -> &mut CompressionInfo;

    // --- Size ---

    /// Runtime serialized size
    fn size(&self) -> usize;

    // --- Pack/Unpack ---

    /// Convert to packed form (Pubkeys -> indices into remaining_accounts)
    fn pack(&self, accounts: &mut PackedAccounts) -> Result<Self::Packed, ProgramError>;

    /// Convert from packed form (indices -> Pubkeys from remaining_accounts)
    fn unpack(packed: &Self::Packed, accounts: &[AccountInfo]) -> Result<Self, ProgramError>;
}
```

---

## Example

### Input

```rust
use light_sdk::LightAccount;

#[derive(LightAccount)]
pub struct UserRecord {
    pub owner: Pubkey,
    pub score: u64,
    pub compression_info: CompressionInfo,
}
```

### Generated

```rust
// Packed struct: Pubkeys -> u8 indices, compression_info EXCLUDED (saves 24 bytes)
pub struct PackedUserRecord {
    pub owner: u8,
    pub score: u64,
    // Note: compression_info is NOT included in packed struct
}

impl LightAccount for UserRecord {
    type Packed = PackedUserRecord;

    const DISCRIMINATOR: [u8; 8] = {
        // SHA256("light:UserRecord")[..8]
        let hash = sha256(b"light:UserRecord");
        [hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7]]
    };

    const INIT_SPACE: usize = 32 + 8 + 1 + std::mem::size_of::<CompressionInfo>();

    fn hash<H: Hasher>(&self) -> Result<[u8; 32], HasherError> {
        let bytes = self.try_to_vec().map_err(|_| HasherError::BorshError)?;
        H::hash(&bytes)
    }

    fn compression_info(&self) -> &CompressionInfo {
        &self.compression_info
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        &mut self.compression_info
    }

    fn size(&self) -> usize {
        self.try_to_vec().map(|v| v.len()).unwrap_or(0)
    }

    fn pack(&self, accounts: &mut PackedAccounts) -> Result<Self::Packed, ProgramError> {
        // compression_info is excluded from packed data (saves 24 bytes)
        Ok(PackedUserRecord {
            owner: accounts.insert_or_get(self.owner),
            score: self.score,
        })
    }

    fn unpack(packed: &Self::Packed, accounts: &[AccountInfo]) -> Result<Self, ProgramError> {
        Ok(UserRecord {
            owner: *accounts.get(packed.owner as usize)
                .ok_or(ProgramError::InvalidAccountData)?.key,
            score: packed.score,
            // Insert canonical CompressionInfo::compressed() for hash verification
            compression_info: CompressionInfo::compressed(),
        })
    }
}
```

---

## Required Field

The struct **must** have a `compression_info` field:

```rust
pub struct MyAccount {
    pub data: u64,
    pub compression_info: CompressionInfo,  // Required
}
```

---

## compression_info Handling

The `compression_info` field is handled specially to optimize instruction data size while maintaining hash consistency:

### Design

1. **Excluded from packed data**: Saves 24 bytes per account in instruction data
2. **Canonical value for hashing**: Both compress and decompress use `CompressionInfo::compressed()`
3. **Runtime values populated separately**: Actual values (last_claimed_slot, rent_config, etc.) set from program config

### Flow

```
CLIENT (pack):
  UserRecord { compression_info: ..., owner, score }
       |
       v
  PackedUserRecord { owner: u8, score }  // compression_info EXCLUDED
       |
       v
  [instruction data - 24 bytes smaller]

PROGRAM (unpack):
  PackedUserRecord { owner, score }
       |
       v
  UserRecord {
      compression_info: CompressionInfo::compressed(),  // CANONICAL value inserted
      owner: Pubkey,
      score
  }
       |
       v
  try_to_vec() --> Sha256BE::hash()  // Hash includes canonical compression_info
```

### Canonical Value

`CompressionInfo::compressed()` returns a canonical value used for hashing:

```rust
CompressionInfo {
    last_claimed_slot: 0,
    lamports_per_write: 0,
    config_version: 0,
    state: CompressionState::Compressed,
    _padding: 0,
    rent_config: RentConfig::default(),  // All zeros
}
```

Both compress and decompress operations use this same canonical value, ensuring hash consistency.

### Runtime Values

After hash verification during decompression, the actual runtime values are populated:

```rust
// After hash verification
account.set_decompressed(config, current_slot);  // Populates from program config
```

---

## Supported Attributes

### `#[compress_as(field = expr)]` - Field Overrides

Override field values in compressed representation:

```rust
#[derive(LightAccount)]
#[compress_as(cached_value = 0)]
pub struct GameSession {
    pub session_id: u64,
    pub cached_value: u64,    // Will be 0 in compressed/hashed form
    pub compression_info: CompressionInfo,
}
```

### `#[skip]` - Exclude Fields

Exclude fields from compression and size calculations:

```rust
#[derive(LightAccount)]
pub struct CachedData {
    pub id: u64,
    #[skip]
    pub local_cache: u64,     // Not included in hash or packed form
    pub compression_info: CompressionInfo,
}
```

---

## SDK Pre-implemented: TokenData

The SDK provides `LightAccount` implementation for token data:

```rust
// In light_sdk (pre-implemented, not generated)
impl LightAccount for TokenData {
    type Packed = PackedTokenData;

    const DISCRIMINATOR: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 4];  // TokenDataVersion::ShaFlat
    const INIT_SPACE: usize = 32 + 32 + 8 + 33 + 1 + 64;

    fn hash<H: Hasher>(&self) -> Result<[u8; 32], HasherError> {
        // SHA256 flat hash for tokens
        self.hash_sha_flat()
    }

    // TokenData does not have compression_info - tokens are always compressed
    fn compression_info(&self) -> &CompressionInfo { unimplemented!() }
    fn compression_info_mut(&mut self) -> &mut CompressionInfo { unimplemented!() }

    fn size(&self) -> usize { ... }
    fn pack(&self, accounts: &mut PackedAccounts) -> Result<Self::Packed, ProgramError> { ... }
    fn unpack(packed: &Self::Packed, accounts: &[AccountInfo]) -> Result<Self, ProgramError> { ... }
}
```

```rust
pub struct TokenData {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
    pub delegate: Option<Pubkey>,
    pub state: AccountState,
    pub tlv: Option<Vec<ExtensionStruct>>,
}

pub struct PackedTokenData {
    pub mint: u8,
    pub owner: u8,
    pub amount: u64,
    pub delegate: u8,
    pub has_delegate: bool,
    pub version: u8,
}
```

---

## SDK Pre-implemented: MintData

```rust
impl LightAccount for MintData {
    type Packed = PackedMintData;
    // ... similar implementation
}

pub struct MintData {
    pub mint_authority: Option<Pubkey>,
    pub supply: u64,
    pub decimals: u8,
    pub is_initialized: bool,
    pub freeze_authority: Option<Pubkey>,
}
```

---

## Relationship to LightAccountVariant

`LightAccount` is for **data structs**. `LightAccountVariant` combines data with seeds:

```rust
// LightAccount: just the data
impl LightAccount for UserRecord { ... }

// LightAccountVariant: data + seeds
pub struct UserRecordVariant {
    pub seeds: UserRecordSeeds,
    pub data: UserRecord,  // <-- implements LightAccount
}

impl LightAccountVariant for UserRecordVariant {
    type Data = UserRecord;  // <-- LightAccount bound
    ...
}
```

---

## Summary

| What | Generated By |
|------|--------------|
| `PackedUserRecord` struct | `#[derive(LightAccount)]` |
| `impl LightAccount for UserRecord` | `#[derive(LightAccount)]` |
| `impl LightAccount for TokenData` | SDK (pre-implemented) |
| `impl LightAccount for MintData` | SDK (pre-implemented) |

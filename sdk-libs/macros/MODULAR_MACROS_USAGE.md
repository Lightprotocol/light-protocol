# Modular Compressible Account Macros

This document demonstrates how to use the new modular macros to replace manual trait implementations for compressible accounts.

## Available Macros

### 1. `#[derive(Compressible)]`

**Generates:** `HasCompressionInfo`, `Size`, and `CompressAs` trait implementations.

**Replaces:** All manual trait implementations for individual account types.

**Usage:**

```rust
use light_sdk_macros::Compressible;
use light_sdk::compressible::CompressionInfo;

// Basic usage - keeps all fields as-is during compression
#[derive(Compressible)]
pub struct UserRecord {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    #[hash]
    pub owner: Pubkey,
    pub name: String,
    pub score: u64,
}

// Custom compression - reset specific fields
#[derive(Compressible)]
#[compress_as(start_time = 0, end_time = None, score = 0)]
pub struct GameSession {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    pub session_id: u64,        // KEPT
    pub player: Pubkey,         // KEPT
    pub game_type: String,      // KEPT
    pub start_time: u64,        // RESET to 0
    pub end_time: Option<u64>,  // RESET to None
    pub score: u64,             // RESET to 0
}
```

### 2. `#[derive(CompressiblePack)]`

**Generates:** `Pack` and `Unpack` trait implementations, plus `PackedXxx` struct for types with Pubkeys.

**Replaces:** All manual `Pack`/`Unpack` implementations and `PackedXxx` struct definitions.

**Usage:**

```rust
use light_sdk_macros::CompressiblePack;

#[derive(CompressiblePack)]
pub struct UserRecord {
    pub compression_info: Option<CompressionInfo>,
    pub owner: Pubkey,  // Will be packed as u8 index
    pub name: String,   // Kept as-is
    pub score: u64,     // Kept as-is
}
// Automatically generates PackedUserRecord struct + all Pack/Unpack impls
```

### 3. `compressed_account_variant!` macro

**Generates:** `CompressedAccountVariant` enum + all trait implementations + `CompressedAccountData` struct.

**Replaces:** Entire enum definition and all its trait implementations.

**Usage:**

```rust
use light_sdk_macros::compressed_account_variant;

// Generate the unified enum for all account types
compressed_account_variant!(UserRecord, GameSession, PlaceholderRecord);
```

## Complete Example: Replacing Manual Implementation

### Before (Manual Implementation):

```rust
// Manual trait implementations for each account type
impl HasCompressionInfo for UserRecord { /* 20+ lines */ }
impl Size for UserRecord { /* 3 lines */ }
impl CompressAs for UserRecord { /* 10+ lines */ }
impl Pack for UserRecord { /* 10+ lines */ }
impl Unpack for UserRecord { /* 5+ lines */ }

// Manual PackedUserRecord struct
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct PackedUserRecord { /* fields */ }
impl Pack for PackedUserRecord { /* 5+ lines */ }
impl Unpack for PackedUserRecord { /* 10+ lines */ }

// Repeat for GameSession, PlaceholderRecord...

// Manual CompressedAccountVariant enum + all traits (100+ lines)
#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum CompressedAccountVariant { /* variants */ }
impl Default for CompressedAccountVariant { /* match arms */ }
impl DataHasher for CompressedAccountVariant { /* match arms */ }
// ... 5 more trait implementations

// Manual CompressedAccountData struct
#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct CompressedAccountData { /* fields */ }
```

### After (Using Modular Macros):

```rust
use light_sdk_macros::{Compressible, CompressiblePack, compressed_account_variant};

// Account definitions with automatic trait generation
#[derive(Compressible, CompressiblePack)]
pub struct UserRecord {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    #[hash]
    pub owner: Pubkey,
    pub name: String,
    pub score: u64,
}

#[derive(Compressible, CompressiblePack)]
#[compress_as(start_time = 0, end_time = None, score = 0)]
pub struct GameSession {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    pub session_id: u64,
    #[hash]
    pub player: Pubkey,
    pub game_type: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub score: u64,
}

#[derive(Compressible, CompressiblePack)]
pub struct PlaceholderRecord {
    #[skip]
    pub compression_info: Option<CompressionInfo>,
    #[hash]
    pub owner: Pubkey,
    pub name: String,
    pub placeholder_id: u64,
}

// Generate the unified enum and data structures
compressed_account_variant!(UserRecord, GameSession, PlaceholderRecord);
```

## Code Reduction

- **Manual implementation**: ~500+ lines of boilerplate trait implementations
- **Macro implementation**: ~15 lines with derive attributes + 1 macro call
- **Reduction**: ~97% less boilerplate code
- **Maintainability**: Single source of truth for trait implementations
- **Consistency**: Guaranteed identical behavior across all account types

## Benefits

1. **Drop-in Replacement**: Each macro replaces specific manual code sections
2. **Modular**: Can use macros independently (e.g., just `#[derive(Compressible)]`)
3. **Configurable**: Custom compression behavior via `compress_as` attribute
4. **Type Safety**: Compile-time validation of all trait implementations
5. **Future-Proof**: Centralized logic that's easy to update

## Migration Guide

1. **Replace individual trait impls**: Add `#[derive(Compressible)]` to account structs
2. **Replace Pack/Unpack impls**: Add `#[derive(CompressiblePack)]` to account structs
3. **Replace enum + traits**: Replace entire enum with `compressed_account_variant!` macro call
4. **Remove manual code**: Delete all manual trait implementations and structs
5. **Test**: Verify identical behavior with existing tests

The macros generate identical code to the manual implementation, ensuring 100% compatibility.

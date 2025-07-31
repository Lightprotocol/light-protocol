# Zero-Copy Compressible Support

This document explains how to use the new zero-copy compressible support in the Light Protocol SDK macros.

## Overview

The SDK now supports two types of compressible accounts:

1. **Regular Compressible**: Uses `Option<CompressionInfo>` and `#[derive(Compressible)]`
2. **Zero-Copy Compressible**: Uses `ZeroCopyCompressionInfo` and `#[derive(ZeroCopyCompressible)]`

## Usage

### 1. Define Zero-Copy Compressible Accounts

```rust
use anchor_lang::prelude::*;
use light_sdk::compressible::ZeroCopyCompressionInfo;
use light_sdk_macros::ZeroCopyCompressible;

#[derive(ZeroCopyCompressible, Clone, Copy)]
#[account(zero_copy(unsafe))]
#[repr(C, packed)]
pub struct MyZeroCopyAccount {
    pub owner: Pubkey,
    pub data: u64,
    pub compression_info: ZeroCopyCompressionInfo,
}

impl Default for MyZeroCopyAccount {
    fn default() -> Self {
        Self {
            owner: Pubkey::default(),
            data: 0,
            compression_info: ZeroCopyCompressionInfo::none(),
        }
    }
}

impl MyZeroCopyAccount {
    pub const LEN: usize = 32 + 8 + 16; // pubkey + u64 + ZeroCopyCompressionInfo
}
```

### 2. Use in Program with Macro

```rust
use anchor_lang::prelude::*;
use light_sdk_macros::add_compressible_instructions;

declare_id!("YourProgramId11111111111111111111111111111");

#[add_compressible_instructions(ZeroCopy(MyZeroCopyAccount))]
#[program]
pub mod my_program {
    use super::*;

    // Your regular instructions here
    pub fn create_account(ctx: Context<CreateAccount>) -> Result<()> {
        // Your logic here
        Ok(())
    }
}
```

### 3. Mixed Usage (Not Yet Supported)

```rust
// This will produce a compile error:
// #[add_compressible_instructions(RegularAccount, ZeroCopy(ZeroCopyAccount))]
//
// Instead, use separate macros or ensure all accounts use the same type:
#[add_compressible_instructions(ZeroCopy(Account1), ZeroCopy(Account2))]
```

## Key Differences

### Regular Compressible

```rust
#[derive(Compressible)]
pub struct RegularAccount {
    pub data: u64,
    pub compression_info: Option<CompressionInfo>, // 10 bytes
}
```

### Zero-Copy Compressible

```rust
#[derive(ZeroCopyCompressible)]
#[account(zero_copy(unsafe))]
#[repr(C, packed)]
pub struct ZeroCopyAccount {
    pub data: u64,
    pub compression_info: ZeroCopyCompressionInfo, // 16 bytes, fixed size
}
```

## Memory Layout

- `Option<CompressionInfo>`: Variable size (1 byte discriminant + 9 bytes when Some)
- `ZeroCopyCompressionInfo`: Fixed 16 bytes, zero-copy compatible

```
ZeroCopyCompressionInfo layout:
- last_written_slot: u64 (8 bytes)
- state: u8 (1 byte)
- is_present: u8 (1 byte)
- _padding: [u8; 6] (6 bytes)
Total: 16 bytes
```

## Generated Traits

### Regular Compressible generates:

- `impl CompressAs for MyAccount`
- `impl HasCompressionInfo for MyAccount`
- `impl Size for MyAccount`

### Zero-Copy Compressible generates:

- `impl CompressAs for MyAccount`
- `impl HasZeroCopyCompressionInfo for MyAccount`
- `impl Size for MyAccount`

## Safe API Usage

```rust
// Safe access (validates internal state)
let slot = account.compression_info.last_written_slot()?;
let is_compressed = account.compression_info.is_compressed()?;

// Unchecked access (faster, but assumes valid state)
let slot = account.compression_info.last_written_slot_unchecked();
let is_compressed = account.compression_info.is_compressed_unchecked();

// Setting values
account.compression_info.set_some_decompressed()?;
account.compression_info.set_compressed()?;
account.compression_info.set_none();
```

## Migration Guide

To migrate from regular compressible to zero-copy:

1. Change the compression_info field type:

   ```rust
   // From:
   pub compression_info: Option<CompressionInfo>,

   // To:
   pub compression_info: ZeroCopyCompressionInfo,
   ```

2. Update the derive macro:

   ```rust
   // From:
   #[derive(Compressible)]

   // To:
   #[derive(ZeroCopyCompressible)]
   ```

3. Add zero-copy attributes:

   ```rust
   #[account(zero_copy(unsafe))]
   #[repr(C, packed)]
   ```

4. Update the add_compressible_instructions macro:

   ```rust
   // From:
   #[add_compressible_instructions(MyAccount)]

   // To:
   #[add_compressible_instructions(ZeroCopy(MyAccount))]
   ```

5. Update Default implementation:

   ```rust
   // From:
   compression_info: None,

   // To:
   compression_info: ZeroCopyCompressionInfo::none(),
   ```

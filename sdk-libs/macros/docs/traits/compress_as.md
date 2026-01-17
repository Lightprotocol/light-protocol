# CompressAs Derive Macro

## 1. Overview

The `#[derive(CompressAs)]` macro generates the `CompressAs` trait implementation, which creates a compressed representation of an account struct. This compressed form is used for hashing and storing in the Light Protocol compression system.

**When to use**: Apply this derive when you need only the compression transformation logic. For most use cases, prefer `#[derive(Compressible)]` or `#[derive(LightCompressible)]` which include this trait.

**Source**: `sdk-libs/macros/src/rentfree/traits/traits.rs` (lines 91-153)

---

## 2. How It Works

### 2.1 Compile-Time Flow

```
+---------------------+     +-------------------+     +-------------------+
|    Input Struct     | --> |   Macro at        | --> |   Generated       |
|                     |     |   Compile Time    |     |   Code            |
+---------------------+     +-------------------+     +-------------------+
| #[compress_as(      |     | 1. Parse struct   |     | impl CompressAs   |
|   cached = 0)]      |     |    attributes     |     |   for GameData {  |
| pub struct GameData |     | 2. Classify each  |     |   fn compress_as  |
| {                   |     |    field:         |     |     -> Cow<Self>  |
|   score: u64,       |     |    - Skip?        |     |   { ... }         |
|   cached: u64,      |     |    - Override?    |     | }                 |
|   compression_info  |     |    - Copy/Clone?  |     |                   |
| }                   |     | 3. Generate impl  |     |                   |
+---------------------+     +-------------------+     +-------------------+
```

### 2.2 Field Classification

Each struct field is classified at compile time:

```
Field Processing Pipeline
+------------------------+
|     Input Field        |
+------------------------+
          |
          v
+------------------------+     YES     +------------------+
| Is "compression_info"? |------------>| Set to None      |
+------------------------+             +------------------+
          | NO
          v
+------------------------+     YES     +------------------+
| Has #[skip] attr?      |------------>| Exclude entirely |
+------------------------+             +------------------+
          | NO
          v
+------------------------+     YES     +------------------+
| Has #[compress_as]     |------------>| Use override     |
| override?              |             | expression       |
+------------------------+             +------------------+
          | NO
          v
+------------------------+     YES     +------------------+
| Is Copy type?          |------------>| self.field       |
+------------------------+             +------------------+
          | NO
          v
+------------------------+
| self.field.clone()     |
+------------------------+
```

### 2.3 Purpose in Compression System

The compressed representation is used for hashing account state:

```
Original Account               compress_as()              Hash Input
+----------------------+       +----------------------+    +----------+
| score: 100           |       | score: 100           |    |          |
| cached: 999          |  -->  | cached: 0  (zeroed)  | -> | SHA256   |
| last_login: 12345    |       | (skipped)            |    | hash     |
| compression_info:    |       | compression_info:    |    |          |
|   Some(...)          |       |   None               |    |          |
+----------------------+       +----------------------+    +----------+
```

This ensures that:
- Transient fields (caches, timestamps) don't affect the hash
- `compression_info` metadata doesn't affect content hash
- Only semantically meaningful data is included

---

## 3. Generated Trait

The macro implements `light_sdk::compressible::CompressAs`:

```rust
impl CompressAs for YourStruct {
    type Output = Self;

    fn compress_as(&self) -> Cow<'_, Self::Output>;
}
```

The `compress_as()` method returns a `Cow::Owned` containing a copy of the struct with:
- `compression_info` set to `None`
- All other fields copied (Clone for non-Copy types)
- Any `#[compress_as(...)]` overrides applied

---

## 4. Supported Attributes

### `#[compress_as(field = expr, ...)]` - Field Overrides

Override specific field values in the compressed representation. Useful for zeroing out fields that shouldn't affect the compressed hash.

```rust
#[derive(CompressAs)]
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

Mark fields to exclude from the compressed representation entirely:

```rust
#[derive(CompressAs)]
pub struct CachedData {
    pub id: u64,
    #[skip]  // Not included in compress_as output
    pub cached_timestamp: u64,
    pub compression_info: Option<CompressionInfo>,
}
```

---

## 5. Auto-Skipped Fields

The following fields are automatically excluded from compression:
- `compression_info` - Always handled specially (set to `None`)
- Fields marked with `#[skip]`

---

## 6. Code Example

### Input

```rust
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::CompressAs;

#[derive(Clone, CompressAs)]
#[compress_as(cached_score = 0)]
pub struct UserRecord {
    pub owner: Pubkey,
    pub score: u64,
    pub cached_score: u64,  // Overridden to 0
    #[skip]
    pub last_updated: u64,  // Excluded entirely
    pub compression_info: Option<CompressionInfo>,
}
```

### Generated Output

```rust
impl light_sdk::compressible::CompressAs for UserRecord {
    type Output = Self;

    fn compress_as(&self) -> std::borrow::Cow<'_, Self::Output> {
        std::borrow::Cow::Owned(Self {
            compression_info: None,
            owner: self.owner,           // Copy type - direct copy
            score: self.score,           // Copy type - direct copy
            cached_score: 0,             // Override from #[compress_as]
            // last_updated skipped due to #[skip]
        })
    }
}
```

---

## 7. Copy vs Clone Behavior

The macro automatically detects Copy types and handles them efficiently:

| Type | Behavior |
|------|----------|
| Copy types (`u8`, `u64`, `Pubkey`, etc.) | Direct copy: `self.field` |
| Non-Copy types (`String`, `Vec`, etc.) | Clone: `self.field.clone()` |

Copy types recognized:
- Primitives: `bool`, `u8`-`u128`, `i8`-`i128`, `f32`, `f64`, `char`
- Solana types: `Pubkey`
- Arrays of Copy types

---

## 8. Usage Notes

- The struct must implement `Clone` for non-Copy field types
- Field overrides in `#[compress_as(...)]` must be valid expressions for the field type
- The `compression_info` field is required but does not need to be specified in overrides

---

## 9. Related Macros

| Macro | Relationship |
|-------|--------------|
| [`HasCompressionInfo`](has_compression_info.md) | Provides compression info accessors (used alongside) |
| [`Compressible`](compressible.md) | Includes `CompressAs` + other compression traits |
| [`LightCompressible`](light_compressible.md) | Includes all compression traits including `CompressAs` |

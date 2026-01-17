# HasCompressionInfo Derive Macro

## 1. Overview

The `#[derive(HasCompressionInfo)]` macro generates accessor methods for the `compression_info` field on compressible account structs. This trait is required for the Light Protocol compression system to read and write compression metadata.

**When to use**: Apply this derive when you need only the compression info accessors, without the full `Compressible` or `LightCompressible` derives.

**Source**: `sdk-libs/macros/src/rentfree/traits/traits.rs` (lines 46-88)

---

## 2. How It Works

### 2.1 Compile-Time Flow

```
+------------------+     +-------------------+     +------------------+
|   Input Struct   | --> |   Macro at        | --> |   Generated      |
|                  |     |   Compile Time    |     |   Code           |
+------------------+     +-------------------+     +------------------+
| pub struct User {|     | 1. Find field     |     | impl HasCompres- |
|   owner: Pubkey, |     |    "compression_  |     |   sionInfo for   |
|   compression_   |     |    info"          |     |   User { ... }   |
|   info: Option<  |     | 2. Validate type  |     |                  |
|   CompressionInfo|     | 3. Generate impl  |     |                  |
| }                |     |                   |     |                  |
+------------------+     +-------------------+     +------------------+
```

### 2.2 Processing Steps

1. **Field Extraction**: Macro extracts all named fields from the struct
2. **Validation**: Searches for `compression_info` field, errors if missing
3. **Code Generation**: Generates trait impl with hardcoded field access

### 2.3 Runtime Behavior

The generated methods provide access to compression metadata stored in the account:

```
Account State                    Method Call
+------------------------+       +------------------------+
| compression_info: Some |  -->  | compression_info()     |
|   address: [u8; 32]    |       | Returns &CompressionInfo
|   lamports: u64        |       +------------------------+
|   ...                  |
+------------------------+       +------------------------+
| compression_info: None |  -->  | compression_info()     |
|                        |       | PANICS!                |
+------------------------+       +------------------------+
                                 | compression_info_mut_  |
                                 | opt() - safe access    |
                                 +------------------------+
```

---

## 3. Generated Trait

The macro implements `light_sdk::compressible::HasCompressionInfo`:

```rust
impl HasCompressionInfo for YourStruct {
    fn compression_info(&self) -> &CompressionInfo;
    fn compression_info_mut(&mut self) -> &mut CompressionInfo;
    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo>;
    fn set_compression_info_none(&mut self);
}
```

### Method Details

| Method | Returns | Description |
|--------|---------|-------------|
| `compression_info()` | `&CompressionInfo` | Returns reference to compression info, panics if `None` |
| `compression_info_mut()` | `&mut CompressionInfo` | Returns mutable reference, panics if `None` |
| `compression_info_mut_opt()` | `&mut Option<CompressionInfo>` | Returns mutable reference to the `Option` itself |
| `set_compression_info_none()` | `()` | Sets the field to `None` |

---

## 4. Required Field

The struct **must** have a field named `compression_info` of type `Option<CompressionInfo>`:

```rust
pub struct MyAccount {
    pub data: u64,
    pub compression_info: Option<CompressionInfo>,  // Required
}
```

If this field is missing, the macro will emit a compile error:

```
error: Struct must have a 'compression_info' field of type Option<CompressionInfo>
```

---

## 5. Code Example

### Input

```rust
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::HasCompressionInfo;

#[derive(HasCompressionInfo)]
pub struct UserRecord {
    pub owner: Pubkey,
    pub score: u64,
    pub compression_info: Option<CompressionInfo>,
}
```

### Generated Output

```rust
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
```

---

## 6. Usage Notes

- The `compression_info()` and `compression_info_mut()` methods will panic if called when the field is `None`. Use `compression_info_mut_opt()` for safe access.
- This trait is automatically included when using `#[derive(Compressible)]` or `#[derive(LightCompressible)]`.
- The field must be named exactly `compression_info` (not `info`, `compress_info`, etc.).

---

## 7. Related Macros

| Macro | Relationship |
|-------|--------------|
| [`Compressible`](compressible.md) | Includes `HasCompressionInfo` + `CompressAs` + `Size` + `CompressedInitSpace` |
| [`LightCompressible`](light_compressible.md) | Includes all compression traits |
| [`CompressAs`](compress_as.md) | Uses `HasCompressionInfo` to access compression metadata |

# Light-Zero-Copy-Derive

Procedural macros for borsh compatible zero copy serialization.

## Main Macros

- `ZeroCopy`: Derives ZeroCopyAt
- `ZeroCopyMut`: Derives ZeroCopyAtMut, ZeroCopyNew
- `ZeroCopyEq`: Derives PartialEq for <StructName>::ZeroCopy == StructName

## Macro Rules

1. Create zero copy structs Z<StructName> for the struct
   1.1. The first consecutive fixed-size fields are extracted into a meta struct Z<StructName>Meta
   1.2. Meta extraction stops at first Vec, Option, or non-Copy type
   1.3. Primitive types are converted to little-endian equivalents (u16→U16, u32→U32, u64→U64, bool→u8)
   1.4. Fields after meta are included directly in the Z-struct and deserialized sequentially
   1.5. Vec<u8> uses optimized slice operations, other Vec<T> types use ZeroCopySlice
   1.6. Option<u64/u32/u16> are optimized, other Option<T> delegate to T's ZeroCopyAt
   1.7. Non-Copy types must implement ZeroCopyAt trait

## Supported Types

### Primitives
- **Unsigned integers**: u8, u16, u32, u64
- **Signed integers**: i8, i16, i32, i64
- **Boolean**: bool

### Collections
- Vec<T> where T is a supported type
- Arrays [T; N] where T is a supported type
- Option<T> where T is a supported type (optimized for u16/u32/u64)

### Custom Types
- Any type that implements ZeroCopyAt trait
- Nested structs with #[derive(ZeroCopy)]
- Enums with unit variants or single unnamed field variants

## Limitations

### Type Support
- **usize/isize**: Platform-dependent size types are not supported for cross-platform consistency
- **f32/f64**: Floating point types are not supported
- **char**: Character type is not supported

### Structural Limitations
- **Tuple structs**: Not supported - only structs with named fields are allowed
- **Empty structs**: Not supported - structs must have at least one field for zero-copy serialization
- **Enum support**:
  - `ZeroCopy` supports enums with unit variants or single unnamed field variants
  - `ZeroCopyMut` does NOT support enums (structs only)
  - `ZeroCopyEq` does NOT support enums (structs only)

### Special Type Handling
- **Arrays in Vec**: `Vec<[T; N]>` is supported. Arrays are Copy types that don't implement the `ZeroCopyStructInner` trait, so they are handled directly after type conversion (e.g., `[u32; N]` → `[U32; N]`) rather than through the trait's associated type.
- **Primitive type conversion**: Integer types are automatically converted to their aligned equivalents for zero-copy safety (e.g., `u32` → `U32`, `i64` → `I64`)

## Requirements

- All structs and enums must have `#[repr(C)]` attribute for memory layout safety
- Fields must implement appropriate traits (Copy for meta fields, ZeroCopyAt for others)

## Basic Usage

```rust
use light_zero_copy_derive::ZeroCopy;
#[derive(ZeroCopy)]
pub struct MyStruct {
    pub a: u8,
}
```

To derive PartialEq as well, use ZeroCopyEq in addition to ZeroCopy:

```rust
use light_zero_copy_derive::{ZeroCopy, ZeroCopyEq};
#[derive(ZeroCopy, ZeroCopyEq)]
pub struct MyStruct {
      pub a: u8,
}
```
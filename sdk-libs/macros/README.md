# Light Protocol SDK Macros

A collection of procedural macros for the Light Protocol SDK.

## LightHasher

The `LightHasher` derive macro implements cryptographic hashing for struct types,
providing implementations of the `ToByteArray` and `DataHasher` traits.

### Attributes

- `#[hash]`: Truncates field data to BN254 field size (for large types)
- `#[skip]`: Ignores field during hashing

### Example

```rust
#[derive(LightHasher, Clone)]
pub struct MyNestedStruct {
    pub a: i32,
    pub b: u32,
    #[hash]
    pub c: String,
}

#[derive(LightHasher, Clone)]
pub struct MyAccount {
    pub a: bool,
    pub b: u64,
    pub c: MyNestedStruct,
    #[hash]
    pub d: [u8; 32],
    pub f: Option<usize>,
}
```

### Debug

```bash
RUST_BACKTRACE=1 cargo test
```
Prints DataHasher::hash() inputs.

## Testing

This crate includes a comprehensive test suite:

```bash
# Run all tests
cargo test

# Run fuzzing tests
cargo test --test hasher_fuzz
cargo test --test fuzz_runner
```

## Fuzzing

For deep, comprehensive fuzzing with cargo-fuzz:

```bash
# Install cargo-fuzz (requires nightly Rust)
cargo install cargo-fuzz

# Run the structure generator fuzzer (tests runtime behavior)
cargo +nightly fuzz run struct_generation -- -max_total_time=300

# Run the macro input fuzzer (tests parsing various struct definitions)
cargo +nightly fuzz run macro_input -- -max_total_time=300
```

For more details, see the [fuzzing documentation](fuzz/README.md).

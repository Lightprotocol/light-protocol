# Fuzzing for LightHasher Derive Macro

This directory contains fuzzing tests for the LightHasher derive macro.

## Setup

1. Install cargo-fuzz:
```bash
cargo install cargo-fuzz
```

2. Note that cargo-fuzz requires nightly Rust:
```bash
rustup default nightly    # Switch default to nightly
# OR
rustup install nightly    # Just install nightly
```

## Running the Fuzzers

The repository includes multiple fuzz targets:

### 1. `macro_input` - Tests the macro's input processing directly

This fuzzer generates random struct definitions and passes them to the derive macro's internal implementation.

```bash
# Run the fuzzer
cargo +nightly fuzz run macro_input

# Run with a time limit (e.g., 5 minutes)
cargo +nightly fuzz run macro_input -- -max_total_time=300
```

### 2. `struct_generation` - Tests runtime behavior of generated code

This fuzzer creates properly typed structs with random data and verifies that hashing works correctly.

```bash
# Run the fuzzer
cargo +nightly fuzz run struct_generation

# Run with a time limit and address sanitizer
RUSTFLAGS="-Zsanitizer=address" cargo +nightly fuzz run struct_generation -- -max_total_time=600
```

## Fuzzing Strategy

The fuzzing approach implements a multi-layered strategy:

1. **Property-Based Testing**: Using random input generation to verify invariants
2. **Structural Testing**: Testing struct definitions with various field types and attributes
3. **Runtime Testing**: Ensuring generated code correctly handles various inputs
4. **Edge Case Testing**: Intentionally testing with invalid inputs

## Adding New Fuzz Targets

To add a new fuzz target:

1. Create a new file in `fuzz_targets/`
2. Add a `[[bin]]` entry in `fuzz/Cargo.toml`

## CI Integration

Add the following to your CI workflow:

```yaml
- name: Run fuzzers
  run: |
    cargo install cargo-fuzz
    cd sdk-libs/macros
    cargo +nightly fuzz run macro_input -- -max_total_time=300 -max_len=1232
    cargo +nightly  fuzz run struct_generation -- -max_total_time=300 -max_len=1232
```

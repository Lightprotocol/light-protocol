# Zero-Copy Crate Guidelines

## Commands
- Build: `cargo build`
- Test all: `cargo test`
- Test single: `cargo test test_name` 
- Run with features: `cargo test --features "solana"`
- Check: `cargo check --all-features`

## Code Style
- **Naming**: snake_case for functions/variables, CamelCase for types
- **Imports**: Group standard lib, external crates, then internal modules
- **Features**: Support conditional compilation with "std" and "solana" features
- **Error Handling**: Use `ZeroCopyError` enum with descriptive variants
- **Types**: Leverage Rust's type system with traits and generics for compile-time safety
- **Memory Safety**: Follow zero-copy principles for memory layout and access
- **Testing**: Write tests for both success and failure cases, use `#[should_panic]` appropriately
- **Documentation**: Document public APIs with examples where possible

## Architecture
This crate provides zero-copy data structures optimized for Solana's limited compute budget while maintaining memory safety through Rust's type system.
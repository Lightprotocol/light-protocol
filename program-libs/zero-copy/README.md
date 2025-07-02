
# Light Zero Copy

Zero copy vector and utils for Solana programs.

## Features

The crate supports the following feature flags:

- `std`: Standard library support (default)
- `solana`: Enable Solana program support using solana-program
- `anchor`: Alias to `solana` for backward compatibility
- `pinocchio`: Enable Pinocchio framework support

Only one framework can be enabled at a time. The crate will use the appropriate imports based on the enabled feature.

## Usage

To use with Solana:

```toml
light-zero-copy = { version = "0.1.0", features = ["solana"] }
```

To use with Pinocchio:

```toml
light-zero-copy = { version = "0.1.0", features = ["pinocchio"] }
```

For backward compatibility with Anchor-based projects:

```toml
light-zero-copy = { version = "0.1.0", features = ["anchor"] }
```

### Security Considerations
- do not use on a 32 bit target with length greater than u32
- only length until u64 is supported

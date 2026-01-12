# Native CToken Examples

This program demonstrates how to use compressed tokens (ctokens) from Light Protocol in a native Solana program (no Anchor framework).

## Overview

The program showcases **8 different instructions** that cover the core compressed token operations:

1. **create_cmint** - Create a compressed mint
2. **mint_to_ctoken** - Mint tokens to compressed accounts
3. **create_token_account_invoke** - Create compressible token account (regular authority)
4. **create_token_account_invoke_signed** - Create compressible token account with PDA ownership
5. **create_ata_invoke** - Create compressible associated token account (regular owner)
6. **create_ata_invoke_signed** - Create compressible ATA with PDA ownership
7. **transfer_interface_invoke** - Transfer compressed tokens (regular authority)
8. **transfer_interface_invoke_signed** - Transfer from PDA-owned account

## Implementation Pattern: Builder Pattern from `ctoken` Module

This implementation uses the **builder pattern** from the `light-ctoken-sdk::ctoken` module. This pattern provides a clean, ergonomic API for CPI operations.

### Why Use the Builder Pattern?

The builder pattern offers several advantages:

- **Type Safety**: Compile-time guarantees for account structures
- **Cleaner Code**: No manual instruction building or account ordering
- **Automatic CPI Handling**: The `invoke()` and `invoke_signed()` methods handle all CPI details
- **Self-Documenting**: Account names make it clear what each field represents

### Example: Transfer with Builder Pattern

```rust
// Build the account infos struct
let transfer_accounts = TransferCTokenCpi {
    source: accounts[0].clone(),
    destination: accounts[1].clone(),
    amount: data.amount,
    authority: accounts[2].clone(),
};

// Invoke the transfer - the builder handles instruction creation and CPI
transfer_accounts.invoke()?;
```

### Example: Transfer with PDA Signing (invoke_signed)

```rust
// Derive PDA
let (pda, bump) = Pubkey::find_program_address(&[TOKEN_ACCOUNT_SEED], &ID);

// Build the account infos struct
let transfer_accounts = TransferCTokenCpi {
    source: accounts[0].clone(),
    destination: accounts[1].clone(),
    amount: data.amount,
    authority: accounts[2].clone(),
};

// Invoke with PDA signing
let signer_seeds: &[&[u8]] = &[TOKEN_ACCOUNT_SEED, &[bump]];
transfer_accounts.invoke_signed(&[signer_seeds])?;
```

## Current Implementation Status

### ✅ Fully Implemented (8/8 Instructions)

All instructions use the **builder pattern** from `light-ctoken-sdk::ctoken`:

- **create_cmint** (Instruction 0): Create compressed mint using `CreateCMintCpi::invoke()`
- **mint_to_ctoken** (Instruction 1): Mint tokens to compressed accounts using `MintToCTokenCpi::invoke()`
- **create_token_account_invoke** (Instruction 2): Create compressible token account using `CreateCTokenAccountCpi`
- **create_token_account_invoke_signed** (Instruction 3): Create with PDA ownership using `invoke_signed()`
- **create_ata_invoke** (Instruction 4): Create compressible ATA using `CreateAssociatedTokenAccountCpi`
- **create_ata_invoke_signed** (Instruction 5): Create ATA with PDA ownership using `invoke_signed()`
- **transfer_interface_invoke** (Instruction 6): Transfer using `TransferCTokenCpi::invoke()`
- **transfer_interface_invoke_signed** (Instruction 7): Transfer with PDA signing using `invoke_signed()`

All instructions compile successfully and demonstrate the clean builder pattern API with constructor usage.

## Project Structure

```
ctoken/native/
├── Cargo.toml           # Path dependencies to light-protocol2/sdk-libs
├── Xargo.toml           # Solana BPF build configuration
├── src/
│   └── lib.rs           # Program entrypoint and instruction handlers
└── README.md            # This file
```

## Dependencies

All dependencies use **path references** to `/Users/ananas/dev/light-protocol2/sdk-libs/`:

- `light-ctoken-sdk` → Main SDK with ctoken builder pattern
- `light-ctoken-types` → Type definitions
- `light-sdk` → Core SDK
- `light-sdk-types` → Common types
- `light-program-test` → Testing framework (dev dependency)
- `light-client` → RPC client (dev dependency)

## Building

```bash
# Check compilation
cargo check

# Build for BPF
cargo build-sbf

# Run unit tests
cargo test

# Run integration tests
cargo test-sbf
```

## Key Concepts

### Compressible Token Accounts

Compressible token accounts have a special extension that allows them to be:
- Compressed back into compressed state
- Configured with rent payment mechanisms
- Automatically closed and compressed

### PDA Patterns (invoke_signed)

The `invoke_signed` variants demonstrate how to:
1. Derive a PDA from the program
2. Use the PDA as the authority/owner for token accounts
3. Sign transactions on behalf of the PDA

This is useful for:
- Escrow programs
- Vaults
- Program-controlled liquidity
- Automated market makers

### Builder Pattern Benefits

The `Cpi` structs from the `ctoken` module provide:

1. **invoke()** - For regular CPI calls where the program acts as authority
2. **invoke_signed()** - For PDA-signed CPI calls
3. **instruction()** - For building instructions without immediate invocation

## Next Steps

To complete this example program:

1. Wait for or implement `AccountInfos` builders for:
   - Create compressed mint
   - Mint to compressed
   - Create token account
   - Create associated token account

2. Add comprehensive integration tests using `light-program-test`

3. Create example client code demonstrating how to call each instruction

## References

- [Light Protocol Documentation](https://www.lightprotocol.com/developers)
- [Compressed Token SDK Source](/Users/ananas/dev/light-protocol2/sdk-libs/compressed-token-sdk)
- [CToken Module](/Users/ananas/dev/light-protocol2/sdk-libs/compressed-token-sdk/src/ctoken)

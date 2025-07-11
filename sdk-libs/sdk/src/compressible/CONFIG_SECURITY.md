# Compressible Config Security Model - Solana Best Practices

## Overview

The compressible config system follows Solana's standard security patterns for program configuration. Only the program's upgrade authority can create the initial config, preventing unauthorized parties from hijacking the configuration system.

## Security Architecture

### 1. Initial Config Creation - Program Upgrade Authority Only

Following Solana best practices (as seen in Anchor's ProgramData pattern), config creation requires:

1. **Program Account**: The program being configured
2. **ProgramData Account**: Contains the program's upgrade authority
3. **Upgrade Authority Signer**: Must sign the transaction

This is the standard pattern used by major Solana programs for admin-controlled operations.

### 2. Safe vs Unsafe Functions

The SDK provides two functions for config creation:

#### `create_config` (Recommended - Safe)

```rust
pub fn create_config<'info>(
    config_account: &AccountInfo<'info>,
    update_authority: &AccountInfo<'info>,
    program_data_account: &AccountInfo<'info>,
    // ... other params
) -> Result<(), LightSdkError>
```

- Validates that the signer is the program's upgrade authority
- Requires the program data account to verify authority
- Prevents unauthorized config creation

#### `create_compression_config_unchecked` (Use with Caution)

```rust
pub fn create_compression_config_unchecked<'info>(
    config_account: &AccountInfo<'info>,
    update_authority: &AccountInfo<'info>,
    // ... other params
) -> Result<(), LightSdkError>
```

- Does NOT validate upgrade authority
- Caller MUST implement their own authority validation
- Only use if you have a custom authorization model

### 3. Separation of Concerns

- **Program Upgrade Authority**: Controls who can create the initial config
- **Config Update Authority**: Controls who can update the config after creation
- These can be the same or different accounts

## Implementation Example

### Manual Program (e.g., using native Solana)

```rust
use light_sdk::compressible::{create_config, CompressibleConfig};

pub fn process_create_compression_config_checked(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), LightSdkError> {
    let payer = &accounts[0];
    let config_account = &accounts[1];
    let update_authority = &accounts[2];
    let system_program = &accounts[3];
    let program_data_account = &accounts[4];

    // Safe function - validates upgrade authority
    create_compression_config_checked(
        config_account,
        update_authority,
        program_data_account,
        &rent_recipient,
        &address_space,
        compression_delay,
        payer,
        system_program,
        &program_id,
    )
}
```

### Anchor Program

See `ANCHOR_CONFIG_EXAMPLE.rs` for a complete Anchor implementation.

## Security Checklist

- [ ] Use `create_config` (not `create_compression_config_unchecked`) unless you have specific requirements
- [ ] Pass the correct program data account
- [ ] Ensure the upgrade authority signs the transaction
- [ ] Deploy config immediately after program deployment
- [ ] Consider transferring config update authority to a multisig
- [ ] Monitor config changes

## Common Vulnerabilities

1. **Using `create_compression_config_unchecked` without validation**: Anyone can create config
2. **Delayed config creation**: Attacker can front-run and create config first
3. **Not monitoring config changes**: Compromised keys can modify settings

## Best Practices

1. **Immediate Initialization**: Create config in the same transaction as program deployment when possible
2. **Authority Management**: Use multisigs for production config authorities
3. **Monitoring**: Set up alerts for config changes
4. **Access Control**: Implement additional checks in your program for sensitive operations

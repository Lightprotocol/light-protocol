# Compressible Config in anchor-compressible

This program demonstrates how to use the Light SDK's compressible config system to manage compression parameters globally.

## Overview

The compressible config allows programs to:

- Set global compression parameters (delay, rent recipient, address space)
- Ensure only authorized parties can modify these parameters
- Validate configuration at runtime

## Instructions

### 1. `initialize_compression_config`

Creates the global config PDA. **Can only be called by the program's upgrade authority**.

**Accounts:**

- `payer`: Transaction fee payer
- `config`: Config PDA (derived with seed `"compressible_config"`)
- `program_data`: Program's data account (for upgrade authority validation)
- `authority`: Program's upgrade authority (must sign)
- `system_program`: System program

**Parameters:**

- `compression_delay`: Number of slots to wait before compression is allowed
- `rent_recipient`: Account that receives rent from compressed PDAs
- `address_space`: Address space for compressed accounts

### 2. `update_compression_config`

Updates the config. **Can only be called by the config's update authority**.

**Accounts:**

- `config`: Config PDA
- `authority`: Config's update authority (must sign)

**Parameters (all optional):**

- `new_compression_delay`: New compression delay
- `new_rent_recipient`: New rent recipient
- `new_address_space`: New address space
- `new_update_authority`: Transfer update authority to a new account

### 3. `create_record`

Creates a compressed user record using config values.

**Additional Accounts:**

- `config`: Config PDA
- `rent_recipient`: Must match the config's rent recipient

### 4. `compress_record`

Compresses a PDA using config values.

**Additional Accounts:**

- `config`: Config PDA
- `rent_recipient`: Must match the config's rent recipient

The compression delay from the config is used to determine if enough time has passed since the last write.

## Security Model

1. **Config Creation**: Only the program's upgrade authority can create the initial config
2. **Config Updates**: Only the config's update authority can modify settings
3. **Rent Recipient Validation**: Instructions validate that the provided rent recipient matches the config
4. **Compression Delay**: Enforced based on config value

## Deployment Process

1. Deploy your program
2. **Immediately** call `initialize_compression_config` with the upgrade authority
3. Optionally transfer config update authority to a multisig or DAO
4. Monitor config changes

## Example Usage

See `examples/config_usage.rs` for complete examples.

## Legacy Instructions

The program still supports legacy instructions that use hardcoded values:

- `create_record`: Uses hardcoded `ADDRESS_SPACE` and `RENT_RECIPIENT`
- `compress_record`: Uses hardcoded `COMPRESSION_DELAY`

These are maintained for backward compatibility but new integrations should use the config-based versions.

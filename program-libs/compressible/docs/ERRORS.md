# Error Types

## CompressibleError

Error codes for the compressible crate, using the 19xxx number range.

**Path:** `program-libs/compressible/src/error.rs`

### FailedBorrowRentSysvar

**Error Code:** 19001

**Description:** Failed to borrow the rent sysvar when calculating rent exemption.

**Common Causes:**
- Rent sysvar is not available in the current execution context
- Running in a test environment without proper sysvar setup
- Corrupted or invalid sysvar account

**Resolution:**
- Ensure the rent sysvar is properly initialized in test environments
- For on-chain programs, verify the sysvar is accessible
- Check that the program has proper permissions to read sysvars

**Usage Example:**
```rust
// In get_rent_exemption_lamports function
let rent = solana_program::rent::Rent::get()
    .map_err(|_| CompressibleError::FailedBorrowRentSysvar)?;
```

---

### InvalidState

**Error Code:** 19002

**Description:** The CompressibleConfig account has an invalid state value for the requested operation.

**Common Causes:**
- Attempting to create new accounts with a config in `Deprecated` or `Inactive` state
- Using an `Inactive` config for any operation (claim, withdraw, compress & close)
- Config state field contains an unrecognized value (not 0, 1, or 2)

**Resolution:**
- For account creation: Ensure config state is `Active` (1)
- For other operations: Ensure config state is not `Inactive` (0)
- Contact the config update authority to activate the config if needed

**State Values:**
- `0` - Inactive: Config cannot be used
- `1` - Active: All operations allowed
- `2` - Deprecated: No new accounts, existing operations continue

**Usage Examples:**
```rust
// Account creation requires Active state
config.validate_active()  // Fails with InvalidState if not Active

// Operations require not Inactive
config.validate_not_inactive()  // Fails with InvalidState if Inactive
```

## Error Conversions

**HasherError:** The crate includes automatic conversion from `light_hasher::HasherError` (7xxx error codes). See the light-hasher crate documentation for specific hasher error details.

## Feature-Specific Conversions

The error types support conversion to different program error types based on features:

- **Solana (default):** Converts to `solana_program_error::ProgramError`
- **Anchor:** Converts to `anchor_lang::prelude::ProgramError`
- **Pinocchio:** Converts to `pinocchio::program_error::ProgramError`

All conversions preserve the numeric error code for consistent error tracking.

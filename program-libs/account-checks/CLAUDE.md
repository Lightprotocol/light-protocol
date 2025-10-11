# Summary
- Unified account validation for both solana-program and pinocchio SDKs
- AccountInfoTrait abstraction enabling single codebase across SDK implementations
- Validation checks with 8-byte discriminators for account type safety
- AccountIterator providing detailed error locations (file:line:column)
- Error codes 12006-12021 with automatic ProgramError conversion

# Used in
- `light-compressed-token` - Validates all account inputs in compressed token instructions
- `light-system` - Core validation for compressed account operations
- `light-compressible` - Validates CompressibleConfig accounts and PDAs
- `light-ctoken-types` - Uses AccountInfoTrait for runtime-agnostic account handling
- `light-account-compression` - Merkle tree account validation
- `light-batched-merkle-tree` - Batch operation account checks
- `compressed-token-sdk` - Uses validation helpers in instruction builders
- `light-sdk` - Core SDK account validation utilities
- `light-sdk-pinocchio` - Pinocchio-specific SDK validation
- `light-sdk-types` - Uses AccountInfoTrait for CPI context and tree info
- `light-compressed-token-types` - Uses AccountInfoTrait for instruction account structures

# Navigation
- This file: Overview and module organization
- For detailed documentation on specific components, see the `docs/` directory:
  - `docs/CLAUDE.md` - Navigation guide for detailed documentation
  - `docs/ACCOUNT_INFO_TRAIT.md` - AccountInfoTrait abstraction and implementations
  - `docs/ACCOUNT_CHECKS.md` - Account validation functions and patterns
  - `docs/ACCOUNT_ITERATOR.md` - Enhanced iterator with error reporting
  - `docs/ERRORS.md` - Error codes (12006-12021), causes, and resolutions
  - `docs/DISCRIMINATOR.md` - Discriminator trait for account type identification
  - `docs/PACKED_ACCOUNTS.md` - Index-based account access utility

# Source Code Structure

## Core Types (`src/`)

### Account Abstraction (`account_info/`)
- `account_info_trait.rs` - AccountInfoTrait definition abstracting over SDK differences
  - Unified data access interface (`try_borrow_data`, `try_borrow_mut_data`)
  - PDA derivation functions (`find_program_address`, `create_program_address`)
  - Ownership and permission checks
- `pinocchio.rs` - Pinocchio AccountInfo implementation (feature: `pinocchio`)
- `solana.rs` - Solana AccountInfo implementation (feature: `solana`)
- `test_account_info.rs` - Mock implementation for unit testing (feature: `test-only`)

### Validation Functions (`checks.rs`)
- Account initialization (`account_info_init` - sets discriminator)
- Ownership validation (`check_owner`, `check_program`)
- Permission checks (`check_mut`, `check_non_mut`, `check_signer`)
- Discriminator validation (`check_discriminator`, `set_discriminator`)
- PDA validation (`check_pda_seeds`, `check_pda_seeds_with_bump`)
- Rent exemption checks (`check_account_balance_is_rent_exempt`)
- Combined validators (`check_account_info_mut`, `check_account_info_non_mut`)

### Account Processing (`account_iterator.rs`)
- Sequential account processing with enhanced error messages
- Named account retrieval with automatic validation
- Location tracking for debugging (file:line:column in errors)
- Convenience methods: `next_signer`, `next_mut`, `next_non_mut`
- Optional account handling (`next_option`, `next_option_mut`)

### Account Type Identification (`discriminator.rs`)
- Discriminator trait for 8-byte account type prefixes
- Constant discriminator arrays for compile-time verification
- Integration with zero-copy deserialization

### Dynamic Access (`packed_accounts.rs`)
- Index-based account access for dynamic account sets
- Bounds-checked retrieval with descriptive error messages
- Used for accessing mint, owner, delegate accounts by index

### Error Handling (`error.rs`)
- AccountError enum with 16 variants (codes 12006-12021)
- Automatic conversions to ProgramError for both SDKs
- Pinocchio ProgramError mapping (standard codes 1-11)
- BorrowError conversions for safe data access

## Feature Flags
- `solana` - Enables solana-program AccountInfo implementation
- `pinocchio` - Enables pinocchio AccountInfo implementation
- `test-only` - Enables test utilities and mock implementations
- Default: No features (trait definitions only)

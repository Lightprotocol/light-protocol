# Summary
- Configuration and rent management for compressible compressed token (CToken) accounts
- Provides `CompressibleConfig` account structure for Light Registry program integration
- Implements rent calculation algorithms for determining account compressibility and claimable rent
- Supports multiple serialization features (Anchor, Pinocchio, Borsh) for program compatibility

# Used in
- `light-compressed-token` - Uses CompressibleConfig for account creation, rent claiming, and closing
- `light-ctoken-types` - Imports CompressibleConfig for compressible extension in token accounts
- `light-registry` - Validates CompressibleConfig for compress & close via registry operations
- `compressed-token-sdk` - Uses rent functions in compress & close instruction builders
- `token-client` - Imports rent calculation helpers for test utilities

# Navigation
- This file: Overview and module organization
- For detailed documentation on specific components, see the `docs/` directory
- `docs/CONFIG_ACCOUNT.md` - CompressibleConfig account structure and methods
- `docs/RENT.md` - Rent calculation functions and compressibility checks
- `docs/ERRORS.md` - Error types with codes, causes, and resolutions
- `docs/SOLANA_RENT.md` - Comparison of Solana vs Light Protocol rent systems

# Source Code Structure

## Core Types (`src/`)
- `config.rs` - CompressibleConfig account structure and PDA derivation
  - Anchor/Borsh/Pod serialization
  - State validation methods (`validate_active`, `validate_not_inactive`)
  - PDA derivation (`derive_pda`, `derive_v1_config_pda`)
  - Default initialization for CToken V1 config

- `rent.rs` - Rent calculation functions and RentConfig
  - Rent curve algorithms (`rent_curve_per_epoch`)
  - Compressibility determination (`calculate_rent_and_balance`)
  - Claimable rent calculations (`claimable_lamports`)
  - Close lamport distribution (`calculate_close_lamports`)

- `error.rs` - Error types with numeric codes (19xxx range)
  - FailedBorrowRentSysvar (19001), InvalidState (19002)
  - HasherError propagation from light-hasher (7xxx codes)
  - ProgramError conversions (Anchor, Pinocchio, Solana)

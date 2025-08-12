# Changelog

## [Unreleased]

### Changed

- **BREAKING**: `add_compressible_instructions` macro no longer generates `create_*` instructions:
  - Removed automatic generation of `create_user_record`, `create_game_session`, etc.
  - Developers must implement their own create instructions with custom initialization logic
  - This change recognizes that create instructions typically need custom business logic
- Updated `add_compressible_instructions` macro to align with new SDK patterns:
  - Now generates `create_compression_config` and `update_compression_config` instructions
  - Uses `HasCompressionInfo` trait instead of deprecated `CompressionTiming`
  - `compress_*` instructions validate against config rent recipient
  - `decompress_multiple_pdas` now accepts seeds in `CompressedAccountData`
  - All generated instructions follow the pattern used in `anchor-compressible`
  - Automatically uses Anchor's `INIT_SPACE` for account size calculation (no manual SIZE needed)

### Added

- **MAJOR**: Enhanced external file module support:
  - Comprehensive pattern matching for common AMM/DEX structures (PoolState, Vault, Position, etc.)
  - Explicit seed specification syntax: `#[add_compressible_instructions(PoolState@[POOL_SEED.as_bytes(), amm_config.key().as_ref()])]`
  - Improved import detection for `pub use` statements and CamelCase account structs
  - Intelligent seed inference for 7+ common DeFi patterns (pools, vaults, positions, configs, etc.)
  - Enhanced error messages with debugging info and actionable solutions
  - Support for complex multi-file project structures like Raydium CP-Swap
- Config management support in generated code:
  - `CreateCompressibleConfig` accounts struct
  - `UpdateCompressibleConfig` accounts struct
  - Automatic config validation in create/compress instructions
- `CompressedAccountData` now includes `seeds` field for flexible PDA derivation
- Generated error codes for config validation
- `CompressionInfo` now implements `anchor_lang::Space` trait for automatic size calculation

### Fixed

- External file module parsing that previously threw "External file modules require explicit seed definitions"
- Import resolution for `pub use` statements across multiple files
- Pattern detection for account structs with various naming conventions

### Removed

- Deprecated `CompressionTiming` trait support
- Hardcoded constants (RENT_RECIPIENT, ADDRESS_SPACE, COMPRESSION_DELAY)
- Manual SIZE constant requirement - now uses Anchor's built-in space calculation

## Migration Guide

1. **Implement your own create instructions** (macro no longer generates them):

   ```rust
   #[derive(Accounts)]
   pub struct CreateUserRecord<'info> {
       #[account(mut)]
       pub user: Signer<'info>,
       #[account(
           init,
           payer = user,
           space = 8 + UserRecord::INIT_SPACE,
           seeds = [b"user_record", user.key().as_ref()],
           bump,
       )]
       pub user_record: Account<'info, UserRecord>,
       pub system_program: Program<'info, System>,
   }

   pub fn create_user_record(ctx: Context<CreateUserRecord>, name: String) -> Result<()> {
       let user_record = &mut ctx.accounts.user_record;
       user_record.compression_info = CompressionInfo::new_decompressed()?;
       user_record.owner = ctx.accounts.user.key();
       user_record.name = name;
       user_record.score = 0;
       Ok(())
   }
   ```

2. Update account structs to use `CompressionInfo` field and derive `InitSpace`:

   ```rust
   #[derive(Debug, LightHasher, LightDiscriminator, Default, InitSpace)]
   #[account]
   pub struct UserRecord {
       #[skip]
       pub compression_info: CompressionInfo,
       #[hash]
       pub owner: Pubkey,
       #[max_len(32)]  // Required for String fields
       pub name: String,
       pub score: u64,
   }
   ```

3. Implement `HasCompressionInfo` trait instead of `CompressionTiming`

4. Create config after program deployment:

   ```typescript
   await program.methods
     .createCompressibleConfig(compressionDelay, rentRecipient, addressSpace)
     .rpc();
   ```

5. Update client code to use new instruction names:
   - `create_record` → `create_user_record` (based on struct name)
   - Pass entire struct data instead of individual fields

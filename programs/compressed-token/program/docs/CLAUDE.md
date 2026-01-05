# Documentation Structure

## Overview
This documentation is organized to provide clear navigation through the compressed token program's functionality.

## Structure
- **`CLAUDE.md`** (this file) - Documentation structure guide
- **`../CLAUDE.md`** (parent) - Main entry point with summary and instruction index
- **`ACCOUNTS.md`** - Complete account layouts and data structures
- **`EXTENSIONS.md`** - Token-2022 extension validation across ctoken instructions
- **`INSTRUCTIONS.md`** - Full instruction reference and discriminator table
- **`RESTRICTED_T22_EXTENSIONS.md`** - SPL Token-2022 behavior for 5 restricted extensions
- **`T22_VS_CTOKEN_COMPARISON.md`** - Comparison of T22 vs ctoken extension behavior
- **`compressed_token/`** - Compressed token operations (Merkle tree accounts)
  - `TRANSFER2.md` - Batch transfer with compress/decompress operations
  - `MINT_ACTION.md` - Mint operations and compressed mint management
  - `FREEZE.md` - Freeze compressed token accounts (Anchor)
  - `THAW.md` - Thaw frozen compressed token accounts (Anchor)
  - `CREATE_TOKEN_POOL.md` - Create initial token pool for SPL/T22 mint compression
  - `ADD_TOKEN_POOL.md` - Add additional token pools (up to 5 per mint)
- **`compressible/`** - Rent management for compressible accounts
  - `CLAIM.md` - Claim rent from expired compressible accounts
  - `WITHDRAW_FUNDING_POOL.md` - Withdraw funds from rent recipient pool
- **`ctoken/`** - CToken (decompressed) account operations
  - `CREATE.md` - Create token account & associated token account
  - `CLOSE.md` - Close decompressed token accounts
  - `TRANSFER.md` - Transfer between decompressed accounts
  - `TRANSFER_CHECKED.md` - Transfer with decimals validation
  - `APPROVE.md` - Approve delegate
  - `APPROVE_CHECKED.md` - Approve with decimals validation
  - `REVOKE.md` - Revoke delegate
  - `MINT_TO.md` - Mint tokens to CToken account
  - `MINT_TO_CHECKED.md` - Mint with decimals validation
  - `BURN.md` - Burn tokens from CToken account
  - `BURN_CHECKED.md` - Burn with decimals validation
  - `FREEZE_ACCOUNT.md` - Freeze CToken account
  - `THAW_ACCOUNT.md` - Thaw frozen CToken account

## Navigation Tips
- Start with `../CLAUDE.md` for the instruction index and overview
- Use `ACCOUNTS.md` for account structure reference
- Use `INSTRUCTIONS.md` for discriminator reference and instruction index
- Refer to specific instruction docs for implementation details

# Documentation Structure

## Overview
This documentation is organized to provide clear navigation through the compressed token program's functionality.

## Structure
- **`CLAUDE.md`** (this file) - Documentation structure guide
- **`../CLAUDE.md`** (parent) - Main entry point with summary and instruction index
- **`ACCOUNTS.md`** - Complete account layouts and data structures
- **`EXTENSIONS.md`** - Token-2022 extension validation across ctoken instructions
- **`RESTRICTED_T22_EXTENSIONS.md`** - SPL Token-2022 behavior for 5 restricted extensions
- **`T22_VS_CTOKEN_COMPARISON.md`** - Comparison of T22 vs ctoken extension behavior
- **`instructions/`** - Detailed instruction documentation
  - `CREATE_TOKEN_ACCOUNT.md` - Create token account & associated token account instructions
  - `MINT_ACTION.md` - Mint operations and compressed mint management
  - `TRANSFER2.md` - Batch transfer instruction for compressed/decompressed operations
  - `CLAIM.md` - Claim rent from expired compressible accounts
  - `CLOSE_TOKEN_ACCOUNT.md` - Close decompressed token accounts
  - `CTOKEN_TRANSFER.md` - Transfer between decompressed accounts
  - `CTOKEN_TRANSFER_CHECKED.md` - Transfer with decimals validation
  - `CTOKEN_APPROVE.md` - Approve delegate on decompressed CToken account
  - `CTOKEN_REVOKE.md` - Revoke delegate on decompressed CToken account
  - `CTOKEN_MINT_TO.md` - Mint tokens to decompressed CToken account
  - `CTOKEN_BURN.md` - Burn tokens from decompressed CToken account
  - `CTOKEN_FREEZE_ACCOUNT.md` - Freeze decompressed CToken account
  - `CTOKEN_THAW_ACCOUNT.md` - Thaw frozen decompressed CToken account
  - `CTOKEN_APPROVE_CHECKED.md` - Approve delegate with decimals validation
  - `CTOKEN_MINT_TO_CHECKED.md` - Mint tokens with decimals validation
  - `CTOKEN_BURN_CHECKED.md` - Burn tokens with decimals validation
  - `WITHDRAW_FUNDING_POOL.md` - Withdraw funds from rent recipient pool
  - `CREATE_TOKEN_POOL.md` - Create initial token pool for SPL/T22 mint compression
  - `ADD_TOKEN_POOL.md` - Add additional token pools (up to 5 per mint)

## Navigation Tips
- Start with `../CLAUDE.md` for the instruction index and overview
- Use `ACCOUNTS.md` for account structure reference
- Refer to specific instruction docs for implementation details

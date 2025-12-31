# Wrapped SOL Support

## Overview

The CToken program treats wrapped SOL (native mint) like any other token. There is no special handling for native mints - compressed wrapped SOL accounts behave identically to any other compressed token account.

## Key Points

### Compression

Wrapped SOL from SPL Token or Token-2022 can be compressed like any other token:
- Transfer wrapped SOL from an SPL/T22 token account to the token pool
- Receive compressed token account with the native mint

### is_native Field

The `is_native` field is **always `None`** for CToken accounts, regardless of whether the mint is the native mint (wrapped SOL):
- CToken's `create_token_account` and `create_ata` instructions don't support setting `is_native`
- `CompressedTokenConfig` doesn't include an `is_native` field
- Compressed wrapped SOL accounts are treated identically to any other compressed token

This differs from SPL Token where `is_native = Some(rent_exemption_amount)` for wrapped SOL accounts.

### Wrapping and Unwrapping SOL

To wrap or unwrap SOL, you must use SPL Token or Token-2022 accounts:

**To wrap SOL (SOL → Compressed Wrapped SOL):**
1. Create an SPL/T22 token account for the native mint
2. Transfer SOL to the token account (SPL Token's SyncNative or direct transfer)
3. Compress the wrapped SOL into a compressed token account

**To unwrap SOL (Compressed Wrapped SOL → SOL):**
1. Decompress the compressed wrapped SOL to an SPL/T22 token account
2. Close the SPL/T22 token account to receive SOL

The CToken program does not provide direct wrap/unwrap functionality - these operations require the underlying SPL Token or Token-2022 program.

## Native Mint Address

Both SPL Token and Token-2022 use the same native mint: `So11111111111111111111111111111111111111112`

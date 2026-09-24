# Changelog

All notable changes to this package will be documented in this file.

## 2026-02-27

### Breaking Changes

- `max_top_up` removed from all instruction structs (`Transfer`, `TransferChecked`, `Burn`, `BurnChecked`, `MintTo`, `MintToChecked`, `TransferInterface`, `Approve`, `Revoke`). The on-chain program defaults to `u16::MAX` when not specified.
  Before: `Transfer { max_top_up: Some(u16::MAX), ... }`
  After: `Transfer { ... }` (field removed)
  Migration: remove `max_top_up` from all instruction builders. (#2301)

- `fee_payer` is now required in instruction and CPI APIs. Authority is writable when no `fee_payer` is provided. (#2301)

- `get_token_account_balance()` returns `ProgramError` instead of SDK-specific errors. (#2301)

## 2026-02-18

### Fixes

- `TransferInterfaceCpi` passes `fee_payer` in the LightToLight transfer path. Previously hardcoded to `None`, causing PrivilegeEscalation errors. (#2294)

## 2026-02-17

### Fixes

- `max_top_up` defaults to `u16::MAX` instead of `0` in instruction builders. (#2279)
- Enforces canonical bump in ATA verification. (#2249)

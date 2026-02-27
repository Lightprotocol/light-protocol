# Changelog

All notable changes to this package will be documented in this file.

## 2026-02-27

### Features

- `create_accounts()` generic function for unified PDA, mint, token, and ATA creation. Exported along with `PdaInitParam`, `CreateMintsInput`, `TokenInitParam`, `AtaInitParam`, and `SharedAccounts`. (#2287)

## 2026-02-17

### Fixes

- Enforces canonical bump in ATA verification. (#2249)

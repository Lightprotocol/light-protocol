# Changelog

All notable changes to this package will be documented in this file.

## 2026-02-27

### Features

- 1-byte discriminator support via `#[light_pinocchio(discriminator = [...])]` attribute. Supports variable-length discriminators (1-8 bytes). (#2302)
- `create_accounts()` unified function replaces multiple separate code generation paths for PDAs, mints, tokens, and ATAs. (#2287)
- Forester dashboard with compression improvements. (#2310)

## 2026-02-17

### Features

- `light_program` pinocchio macro refactored for cleaner code generation. (#2247)

### Fixes

- Enforces canonical bump in ATA verification. (#2249)

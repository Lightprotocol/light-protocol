# Changelog

All notable changes to this package will be documented in this file.

## 2026-02-27

### Fixes

- `TransferInterfaceCpi` passes `fee_payer` in the LightToLight transfer path. Previously hardcoded to `None`, causing PrivilegeEscalation errors. (#2294)
- Authority mutability and wire format aligned with token-sdk. (#2301)

## 2026-02-17

### Fixes

- `max_top_up` defaults to `u16::MAX` instead of `0` in instruction builders. (#2279)
- Enforces canonical bump in ATA verification. (#2249)

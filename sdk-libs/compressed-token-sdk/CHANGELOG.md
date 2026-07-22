# Changelog

All notable changes to this package will be documented in this file.

## 2026-02-27

### Breaking Changes

- `max_top_up` removed from instruction structs. Authority mutability and wire format aligned with pinocchio. (#2301)

## 2026-02-17

### Fixes

- `max_top_up` defaults to `u16::MAX` instead of `0` in instruction builders. (#2279)

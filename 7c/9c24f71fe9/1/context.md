# Session Context

## User Prompts

### Prompt 1

Implement the following plan:

# Plan: Fill Coverage Gaps + Refactor Test Helpers

**Date:** 2026-02-20

## IMPORTANT

- Split into todos and work through one by one
- Full `assert_eq!` for all non-error assertions; `assert!(matches!(...))` for errors
- Use `TestAccount` from `light-account-checks` (solana feature)
- No changes to production files — tests and common helpers only

## Context

Two tasks:

1. **Helper consolidation**: `make_valid_accounts` is duplicated word-for-word in all three...


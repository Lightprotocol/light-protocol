# Session Context

## User Prompts

### Prompt 1

Implement the following plan:

# Fix: `assigned_account_index` u8 overflow in `create_accounts()`

**Date:** 2026-02-18
**Branch:** `jorrit/refactor-light-account-creation-to-generic-function`

---

## IMPORTANT

- Single task: two one-line edits to one file
- No new files, no helpers, no test extraction
- After edits: run `cargo check -p light-sdk-types` to confirm compilation

---

## Context

The logic review of `create_accounts()` found that the validation at line 144 of
`sdk-libs/sdk-types/...


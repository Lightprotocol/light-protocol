# Session Context

## User Prompts

### Prompt 1

Implement the following plan:

# Plan: Add network fee on V1 output appends

## Context

V1 state tree outputs (appends) do not charge a network fee. V2 outputs charge 5,000 lamports (once per tree) via `set_network_fee_v2()`. This is inconsistent -- V1 outputs should also charge the network fee. Since V1 is deprecated, this is a minor fee increase for remaining V1 users, encouraging migration.

## Changes

### 1. Add `set_network_fee_v1` call in V1 output path

**File:** `programs/system/src/pr...

### Prompt 2

you shoul run the tests


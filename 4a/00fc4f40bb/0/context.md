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

You are a senior code reviewer ensuring high standards of code quality and security.

When invoked:
1. Take a step back, think hard and be critical.
2. Run `git diff` and `git diff --cached` to see all changes (unstaged and staged) unless instructed otherwise
3. Focus on modified files
4. Create state machine diagrams (internally) to understand the flow:
   - Identify entry points and exit points
   - Map state transitions and decision branches
   - Trace data flow through functions
   - For com...

### Prompt 3

checkout a new branch jorrit/chore-add-v1-tree-deprecation-msg


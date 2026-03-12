# Session Context

## User Prompts

### Prompt 1

Implement the following plan:

# Plan: Bump light-poseidon 0.3 -> 0.4

## IMPORTANT
- Split into individual todos, work through one by one
- Use subagents where it makes sense
- If stuck, use a subagent to research

## Context

light-poseidon 0.4 changes the off-chain input validation from `input.len() > 32` to `input.len() != 32`, requiring all Poseidon hash inputs to be exactly 32 bytes. This aligns with the on-chain enforcement already added in `program-libs/hasher/src/poseidon.rs`. All sites...


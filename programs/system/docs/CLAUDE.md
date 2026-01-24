# Documentation Structure

## Overview
This documentation covers the Light System Program - the core validation and coordination layer for Light Protocol that handles ZK proof verification, CPI context management, and compressed account state transitions.

## Structure
- **`CLAUDE.md`** (this file) - Documentation structure guide
- **`../CLAUDE.md`** (parent) - Main entry point with summary and source code structure
- **`INSTRUCTIONS.md`** - Full instruction reference and discriminator table
- **`ACCOUNTS.md`** - Account layouts and state structures
- **`PROCESSING_PIPELINE.md`** - 19-step processing pipeline documentation
- **`CPI_CONTEXT.md`** - CPI context state management and multi-program transactions
- **`init/`** - CPI context account initialization
  - `INIT_CPI_CONTEXT_ACCOUNT.md` - Initialize CPI context account (version 2)
  - `REINIT_CPI_CONTEXT_ACCOUNT.md` - Migrate from version 1 to version 2
- **`invoke/`** - Direct invocation
  - `INVOKE.md` - Direct invocation instruction
- **`invoke_cpi/`** - CPI invocation modes
  - `INVOKE_CPI.md` - Standard CPI invocation (Anchor mode)
  - `INVOKE_CPI_WITH_READ_ONLY.md` - CPI with read-only account support
  - `INVOKE_CPI_WITH_ACCOUNT_INFO.md` - CPI with dynamic account configuration (V2 mode)

## Navigation Tips
- Start with `../CLAUDE.md` for program overview and source code structure
- Use `INSTRUCTIONS.md` for discriminator reference and instruction index
- Use `ACCOUNTS.md` for CpiContextAccount layout and initialization
- Refer to specific instruction docs for implementation details

| Task | Start Here |
|------|------------|
| Understand program architecture | `../CLAUDE.md` |
| Find instruction discriminators | `INSTRUCTIONS.md` |
| Understand CpiContextAccount layout | `ACCOUNTS.md` |
| Learn 19-step processing flow | `PROCESSING_PIPELINE.md` |
| Multi-program transactions | `CPI_CONTEXT.md` |
| Direct invocation (single program) | `invoke/INVOKE.md` |
| CPI invocation (Anchor mode) | `invoke_cpi/INVOKE_CPI.md` |
| CPI with read-only accounts | `invoke_cpi/INVOKE_CPI_WITH_READ_ONLY.md` |
| CPI with dynamic accounts (V2) | `invoke_cpi/INVOKE_CPI_WITH_ACCOUNT_INFO.md` |
| Initialize CPI context account | `init/INIT_CPI_CONTEXT_ACCOUNT.md` |
| Migrate to V2 CPI context | `init/REINIT_CPI_CONTEXT_ACCOUNT.md` |

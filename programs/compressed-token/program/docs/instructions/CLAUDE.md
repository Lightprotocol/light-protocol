# Documentation Structure

## Overview
This documentation is organized to provide clear navigation through the compressed token program's functionality.

## Structure
- **`CLAUDE.md`** (this file) - Documentation structure guide
- **`../CLAUDE.md`** (parent) - Main entry point with summary and instruction index
- **`ACCOUNTS.md`** - Complete account layouts and data structures
- **`instructions/`** - Detailed instruction documentation
  - `CREATE_TOKEN_ACCOUNT.md` - Create token account & associated token account instructions
  - Additional instruction docs to be added as needed

## Navigation Tips
- Start with `../../CLAUDE.md` for the instruction index and overview
- Use `../ACCOUNTS.md` for account structure reference
- Refer to specific instruction docs for implementation details


# Instructions

**Instruction Schema:**
every instruction description must include the sections:
    - **path** path to instruction code in the program
    - **description** highlevel description what the instruction does including accounts used and their state layout (paths to the code), usage flows what the instruction does
    - **instruction_data** paths to code where instruction data structs are defined
    - **Accounts** accounts in order including checks
    - **instruciton logic and checks**
    - **Errors** possible errors and description what causes these errors

1. create ctoken account & create associated ctoken account (idempotent)
2. transfer2 - batch transfer instruction

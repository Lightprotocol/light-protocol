# Documentation Structure

## Overview
This documentation is organized to provide clear navigation through the compressed token program's functionality.

## Structure
- **`CLAUDE.md`** (this file) - Documentation structure guide
- **`../CLAUDE.md`** (parent) - Main entry point with summary and instruction index
- **`ACCOUNTS.md`** - Complete account layouts and data structures
- **`instructions/`** - Detailed instruction documentation
  - `CREATE_TOKEN_ACCOUNT.md` - Create token account & associated token account instructions
  - `MINT_ACTION.md` - Mint operations and compressed mint management
  - `TRANSFER2.md` - Batch transfer instruction for compressed/decompressed operations
  - `CLAIM.md` - Claim rent from expired compressible accounts
  - `CLOSE_TOKEN_ACCOUNT.md` - Close decompressed token accounts
  - `DECOMPRESSED_TRANSFER.md` - Transfer between decompressed accounts
  - `WITHDRAW_FUNDING_POOL.md` - Withdraw funds from rent recipient pool

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

1. **Create Token Account Instructions** - Create regular and associated ctoken accounts
2. **Transfer2** - Batch transfer instruction supporting compress/decompress/transfer operations
3. **MintAction** - Batch instruction for compressed mint management and mint operations (supports 9 actions: CreateCompressedMint, MintTo, UpdateMintAuthority, UpdateFreezeAuthority, CreateSplMint, MintToCToken, UpdateMetadataField, UpdateMetadataAuthority, RemoveMetadataKey)
4. **Claim** - Rent reclamation from expired compressible accounts
5. **Close Token Account** - Close decompressed token accounts with rent distribution
6. **Decompressed Transfer** - SPL-compatible transfers between decompressed accounts
7. **Withdraw Funding Pool** - Withdraw funds from rent recipient pool

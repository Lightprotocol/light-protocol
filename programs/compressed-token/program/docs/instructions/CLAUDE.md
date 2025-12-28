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
  - `CTOKEN_TRANSFER.md` - Transfer between decompressed accounts
  - `CTOKEN_TRANSFER_CHECKED.md` - Transfer with decimals validation
  - `WITHDRAW_FUNDING_POOL.md` - Withdraw funds from rent recipient pool
  - `CREATE_TOKEN_POOL.md` - Create initial token pool for SPL/T22 mint compression
  - `ADD_TOKEN_POOL.md` - Add additional token pools (up to 5 per mint)
  - `CTOKEN_APPROVE.md` - Approve delegate on decompressed CToken account
  - `CTOKEN_REVOKE.md` - Revoke delegate on decompressed CToken account
  - `CTOKEN_MINT_TO.md` - Mint tokens to decompressed CToken account
  - `CTOKEN_BURN.md` - Burn tokens from decompressed CToken account
  - `CTOKEN_FREEZE_ACCOUNT.md` - Freeze decompressed CToken account
  - `CTOKEN_THAW_ACCOUNT.md` - Thaw frozen decompressed CToken account
  - `CTOKEN_APPROVE_CHECKED.md` - Approve delegate with decimals validation
  - `CTOKEN_MINT_TO_CHECKED.md` - Mint tokens with decimals validation
  - `CTOKEN_BURN_CHECKED.md` - Burn tokens with decimals validation

## Discriminator Reference

| Instruction | Discriminator | Enum Variant |
|-------------|---------------|--------------|
| CTokenTransfer | 3 | `InstructionType::CTokenTransfer` |
| CTokenApprove | 4 | `InstructionType::CTokenApprove` |
| CTokenRevoke | 5 | `InstructionType::CTokenRevoke` |
| CTokenTransferChecked | 6 | `InstructionType::CTokenTransferChecked` |
| CTokenMintTo | 7 | `InstructionType::CTokenMintTo` |
| CTokenBurn | 8 | `InstructionType::CTokenBurn` |
| CloseTokenAccount | 9 | `InstructionType::CloseTokenAccount` |
| CTokenFreezeAccount | 10 | `InstructionType::CTokenFreezeAccount` |
| CTokenThawAccount | 11 | `InstructionType::CTokenThawAccount` |
| CTokenApproveChecked | 12 | `InstructionType::CTokenApproveChecked` |
| CTokenMintToChecked | 14 | `InstructionType::CTokenMintToChecked` |
| CTokenBurnChecked | 15 | `InstructionType::CTokenBurnChecked` |
| CreateTokenAccount | 18 | `InstructionType::CreateTokenAccount` |
| CreateAssociatedCTokenAccount | 100 | `InstructionType::CreateAssociatedCTokenAccount` |
| Transfer2 | 101 | `InstructionType::Transfer2` |
| CreateAssociatedTokenAccountIdempotent | 102 | `InstructionType::CreateAssociatedTokenAccountIdempotent` |
| MintAction | 103 | `InstructionType::MintAction` |
| Claim | 104 | `InstructionType::Claim` |
| WithdrawFundingPool | 105 | `InstructionType::WithdrawFundingPool` |

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
8. **Create Token Pool** - Create initial token pool PDA for SPL/T22 mint compression
9. **Add Token Pool** - Add additional token pools for a mint (up to 5 per mint)
10. **CToken MintTo** - Mint tokens to decompressed CToken account
11. **CToken Burn** - Burn tokens from decompressed CToken account
12. **CToken Freeze/Thaw** - Freeze and thaw decompressed CToken accounts
13. **CToken Approve/Revoke** - Approve and revoke delegate on decompressed CToken accounts
14. **CToken Checked Operations** - ApproveChecked, MintToChecked, BurnChecked with decimals validation

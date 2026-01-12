# Instructions Reference

## Overview
This file contains the discriminator reference table and instruction index for the compressed token program.

## Related Documentation
- **`CLAUDE.md`** - Documentation structure guide
- **`../CLAUDE.md`** (parent) - Main entry point with summary and instruction index
- **`ACCOUNTS.md`** - Complete account layouts and data structures
- **`compressed_token/`** - Compressed token operations (Merkle tree accounts)
  - `TRANSFER2.md` - Batch transfer with compress/decompress operations
  - `MINT_ACTION.md` - Mint operations and compressed mint management
  - `FREEZE.md` - Freeze compressed token accounts (Anchor)
  - `THAW.md` - Thaw frozen compressed token accounts (Anchor)
  - `CREATE_TOKEN_POOL.md` - Create initial token pool for SPL/T22 mint compression
  - `ADD_TOKEN_POOL.md` - Add additional token pools (up to 5 per mint)
- **`compressible/`** - Rent management for compressible accounts
  - `CLAIM.md` - Claim rent from expired compressible accounts
  - `WITHDRAW_FUNDING_POOL.md` - Withdraw funds from rent recipient pool
- **`ctoken/`** - CToken (decompressed) account operations
  - `CREATE.md` - Create token account & associated token account
  - `CLOSE.md` - Close decompressed token accounts
  - `TRANSFER.md` - Transfer between decompressed accounts
  - `TRANSFER_CHECKED.md` - Transfer with decimals validation
  - `APPROVE.md` - Approve delegate
  - `REVOKE.md` - Revoke delegate
  - `MINT_TO.md` - Mint tokens to CToken account
  - `MINT_TO_CHECKED.md` - Mint with decimals validation
  - `BURN.md` - Burn tokens from CToken account
  - `BURN_CHECKED.md` - Burn with decimals validation
  - `FREEZE_ACCOUNT.md` - Freeze CToken account
  - `THAW_ACCOUNT.md` - Thaw frozen CToken account

## Discriminator Reference

| Instruction | Discriminator | Enum Variant | SPL Token Compatible |
|-------------|---------------|--------------|----------------------|
| CTokenTransfer | 3 | `InstructionType::CTokenTransfer` | Transfer |
| CTokenApprove | 4 | `InstructionType::CTokenApprove` | Approve |
| CTokenRevoke | 5 | `InstructionType::CTokenRevoke` | Revoke |
| CTokenMintTo | 7 | `InstructionType::CTokenMintTo` | MintTo |
| CTokenBurn | 8 | `InstructionType::CTokenBurn` | Burn |
| CloseTokenAccount | 9 | `InstructionType::CloseTokenAccount` | CloseAccount |
| CTokenFreezeAccount | 10 | `InstructionType::CTokenFreezeAccount` | FreezeAccount |
| CTokenThawAccount | 11 | `InstructionType::CTokenThawAccount` | ThawAccount |
| CTokenTransferChecked | 12 | `InstructionType::CTokenTransferChecked` | TransferChecked |
| CTokenMintToChecked | 14 | `InstructionType::CTokenMintToChecked` | MintToChecked |
| CTokenBurnChecked | 15 | `InstructionType::CTokenBurnChecked` | BurnChecked |
| CreateTokenAccount | 18 | `InstructionType::CreateTokenAccount` | InitializeAccount3 |
| CreateAssociatedCTokenAccount | 100 | `InstructionType::CreateAssociatedCTokenAccount` | - |
| Transfer2 | 101 | `InstructionType::Transfer2` | - |
| CreateAssociatedTokenAccountIdempotent | 102 | `InstructionType::CreateAssociatedTokenAccountIdempotent` | - |
| MintAction | 103 | `InstructionType::MintAction` | - |
| Claim | 104 | `InstructionType::Claim` | - |
| WithdrawFundingPool | 105 | `InstructionType::WithdrawFundingPool` | - |
| Freeze | Anchor | `anchor_compressed_token::freeze` | - |
| Thaw | Anchor | `anchor_compressed_token::thaw` | - |

**SPL Token Compatibility Notes:**
- Instructions with SPL Token equivalents share the same discriminator and accept the same instruction data format
- CreateTokenAccount (18) accepts 32-byte owner pubkey for InitializeAccount3 compatibility
- CToken-specific instructions (100+) have no SPL Token equivalent

## Navigation Tips
- Start with `../CLAUDE.md` for the instruction index and overview
- Use `ACCOUNTS.md` for account structure reference
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

## Compressed Token Operations (`compressed_token/`)
1. **Transfer2** - Batch transfer instruction supporting compress/decompress/transfer operations
2. **MintAction** - Batch instruction for compressed mint management (9 actions)
3. **Freeze** - Freeze compressed token accounts (Anchor)
4. **Thaw** - Thaw frozen compressed token accounts (Anchor)

## CToken Operations (`ctoken/`)
5. **Create** - Create regular and associated ctoken accounts
6. **Close** - Close decompressed token accounts with rent distribution
7. **Transfer** - SPL-compatible transfers between decompressed accounts
8. **Approve/Revoke** - Approve and revoke delegate on decompressed CToken accounts
9. **MintTo** - Mint tokens to decompressed CToken account
10. **Burn** - Burn tokens from decompressed CToken account
11. **Freeze/Thaw** - Freeze and thaw decompressed CToken accounts
12. **Checked Operations** - TransferChecked, MintToChecked, BurnChecked

## Compressible Operations (`compressible/`)
13. **Claim** - Rent reclamation from expired compressible accounts
14. **Withdraw Funding Pool** - Withdraw funds from rent recipient pool

## Token Pool Operations (root)
15. **Create Token Pool** - Create initial token pool PDA for SPL/T22 mint compression
16. **Add Token Pool** - Add additional token pools for a mint (up to 5 per mint)

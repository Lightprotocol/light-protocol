# Light Protocol: Atomic PDA + Mint Creation via Macros

## Overview

This test program demonstrates how `#[compressible]` PDAs and `#[light_mint]` can be combined in a single instruction with a single proof, enabling atomic creation of compressed accounts and decompressed mints.

## Key Components

### 1. Macro Attributes

**`#[compressible]`** - Applied to PDA account fields:
- `address_tree_info`: Packed address tree info from params
- `output_tree`: State tree index for compressed account output

**`#[light_mint]`** - Applied to mint placeholder fields:
- `mint_signer`: PDA that derives the CMint address
- `authority`: Mint authority (must be signer)
- `decimals`: Mint decimals
- `address_tree_info`: Address tree info for mint's compressed address
- `signer_seeds`: Optional seeds for PDA signing

### 2. Derive Macros

**`#[derive(LightFinalize)]`** - Implements `LightPreInit` and `LightFinalize` traits:
- Detects `#[compressible]` and `#[light_mint]` fields
- Auto-detects ctoken accounts: `ctoken_compressible_config`, `ctoken_rent_sponsor`, `ctoken_program`, `ctoken_cpi_authority`

**`#[light_instruction(params)]`** - Wraps instruction handlers:
- Calls `light_pre_init()` BEFORE instruction body (all compression logic here)
- Calls `light_finalize()` AFTER instruction body (no-op)

### 3. Execution Flow (PDAs + Mint)

```
Instruction Entry
       |
       v
light_pre_init()
       |
       +---> 1. Build CpiAccounts with CPI context
       |
       +---> 2. Prepare compressed account infos for all PDAs (with_data=false)
       |
       +---> 3. write_to_cpi_context_first() - Write PDAs to CPI context
       |
       +---> 4. Build MintActionCompressedInstructionData
       |         - CreateMint with compressed address
       |         - DecompressMintAction (creates CMint on-chain)
       |         - CpiContext config (set_context: false, reads existing)
       |
       +---> 5. Build MintActionMetaConfig with compressible_cmint
       |
       +---> 6. invoke/invoke_signed to ctoken program
       |         - Creates CMint PDA on-chain (DECOMPRESSED/"HOT")
       |         - Registers mint's compressed address
       |         - Light System reads PDAs from CPI context
       |         - All addresses registered atomically
       |
       v
   Return Ok(true)
       |
       v
Instruction Body
   (Can use HOT CMint: mintTo, burn, transfer, etc.)
       |
       v
light_finalize() -> Ok(())  [no-op]
       |
       v
Anchor Exit (serializes all account data)
```

### 4. Key Design Decisions

**All compression in pre_init**: 
- CMint is created and decompressed BEFORE instruction body runs
- Instruction body can immediately use the HOT mint (mintTo, burn, etc.)
- This enables patterns like `raydium-cp-swap` where mint operations follow creation

**with_data=false for PDAs**:
- Compressed account only gets the address (no data hash)
- Actual data stays on-chain PDA with CompressionInfo
- Later auto-compression will fully compress and close the PDA
- SDK enforces this: `with_data=true` throws "not supported yet"

**CPI Context Batching**: When PDAs and mints are combined:
1. PDAs are written to CPI context first via `write_to_cpi_context_first()`
2. Mint action reads from the same CPI context (set_context: false)
3. Light System processes all operations atomically

**Tree Indexing**: Critical for CPI context validation:
- `in_tree_index` is 1-indexed (Light System does `in_tree_index - 1`)
- Points to the state queue, which has `associated_merkle_tree`
- Must match the CPI context's `associated_merkle_tree`

### 5. Required Accounts for Combined Flow

```rust
pub struct CreatePdasAndMintAuto<'info> {
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    pub mint_authority: Signer<'info>,
    pub mint_signer: UncheckedAccount<'info>,      // CMint derives from this
    
    #[compressible(...)]
    pub user_record: Account<'info, UserRecord>,   // PDA to compress
    
    #[compressible(...)]
    pub game_session: Account<'info, GameSession>, // Another PDA
    
    #[light_mint(...)]
    pub lp_mint: UncheckedAccount<'info>,          // CMint placeholder (HOT after pre_init)
    
    pub vault: UncheckedAccount<'info>,            // Program-owned CToken vault
    pub vault_authority: UncheckedAccount<'info>,  // Vault owner PDA
    pub user_ata: UncheckedAccount<'info>,         // User's ATA for lp_mint
    
    pub compression_config: AccountInfo<'info>,    // Light protocol config
    pub ctoken_compressible_config: AccountInfo<'info>,  // Ctoken config
    pub ctoken_rent_sponsor: AccountInfo<'info>,   // Rent sponsor
    pub ctoken_program: AccountInfo<'info>,        // Ctoken program
    pub ctoken_cpi_authority: AccountInfo<'info>,  // Ctoken CPI authority
    pub system_program: Program<'info, System>,
}
```

### 6. Instruction Body: Using the HOT CMint

After `light_pre_init()` creates and decompresses the CMint, the instruction body can immediately use it:

```rust
#[light_instruction(params)]
pub fn create_pdas_and_mint_auto<'info>(ctx: ..., params: ...) -> Result<()> {
    // 1. Populate PDA data (compression handled by macro)
    ctx.accounts.user_record.owner = params.owner;
    ctx.accounts.game_session.session_id = params.session_id;
    
    // 2. Create program-owned CToken vault (like cp-swap's token vaults)
    CreateCTokenAccountCpi {
        payer: ctx.accounts.fee_payer.to_account_info(),
        account: ctx.accounts.vault.to_account_info(),
        mint: ctx.accounts.lp_mint.to_account_info(),  // HOT CMint from pre_init
        owner: ctx.accounts.vault_authority.key(),
        compressible: CompressibleParamsCpi { ... },
    }.invoke_signed(&[vault_seeds])?;
    
    // 3. Create user's ATA (like cp-swap's creator_lp_token)
    CreateAssociatedCTokenAccountCpi {
        owner: ctx.accounts.fee_payer.to_account_info(),
        mint: ctx.accounts.lp_mint.to_account_info(),  // HOT CMint
        associated_token_account: ctx.accounts.user_ata.to_account_info(),
        compressible: CompressibleParamsCpi { ... },
    }.invoke()?;
    
    // 4. Mint tokens to vault and user's ATA
    CTokenMintToCpi {
        cmint: ctx.accounts.lp_mint.to_account_info(),  // HOT CMint
        destination: ctx.accounts.vault.to_account_info(),
        amount: params.vault_mint_amount,
        authority: ctx.accounts.mint_authority.to_account_info(),
    }.invoke()?;
    
    CTokenMintToCpi {
        cmint: ctx.accounts.lp_mint.to_account_info(),  // HOT CMint
        destination: ctx.accounts.user_ata.to_account_info(),
        amount: params.user_ata_mint_amount,
        authority: ctx.accounts.mint_authority.to_account_info(),
    }.invoke()?;
    
    Ok(())
}
```

### 7. Test: `test_create_pdas_and_mint_auto`

Demonstrates the full cp-swap-like flow:
1. Setup compression config and signers
2. Derive PDAs, CMint, vault, and user_ata addresses
3. Get validity proof for all 3 compressed addresses (2 PDAs + 1 mint)
4. Build instruction with CPI context enabled
5. Execute single transaction
6. Verify:
   - 2 PDAs compressed (address only, data on-chain)
   - 1 CMint created and decompressed (HOT)
   - 1 Program-owned vault with correct balance (e.g., 100 tokens)
   - 1 User ATA with correct balance (e.g., 50 tokens)
   - Both vault and ATA owned by ctoken program

## Conclusion

The macro system enables atomic creation of an arbitrary combination of compressed PDAs and decompressed mints in a single instruction with a single proof. All compression logic runs in `light_pre_init()`, so the instruction body can immediately use the HOT CMint for operations like `mintTo`, `burn`, and `transfer`. This pattern is essential for programs like `raydium-cp-swap` where multiple accounts (pool state, observation state, LP mint, token vaults, user ATAs) must be created and operated on atomically.

**The full flow in one instruction:**
1. `pre_init()`: Compress 2 PDAs + Create+Decompress CMint (atomically)
2. `instruction body`: Create vault + Create user_ata + MintTo both
3. `finalize()`: no-op

All accounts (PDAs, CMint, vault, user_ata) exist and are usable within the same instruction.

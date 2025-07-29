# Example Usage

## Basic Usage

```rust
#[add_compressible_instructions(UserRecord, GameSession)]
#[program]
pub mod my_program {
    use super::*;
    // ... your instructions
}
```

## External File Module Support - NEW APPROACH! 🚀

For complex projects with multi-file structures (like Raydium CP-Swap), you can now use the new `derive(Compressible)` approach for **completely automatic seed detection**:

### Step 1: Add derive(Compressible) to your instruction struct

```rust
// instructions/initialize.rs
use anchor_lang::prelude::*;
use light_sdk_macros::Compressible;  // Import the derive macro

#[derive(Accounts, Compressible)]  // ← Add Compressible derive!
pub struct Initialize<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        init,
        seeds = [
            POOL_SEED.as_bytes(),
            amm_config.key().as_ref(),
            token_0_mint.key().as_ref(),
            token_1_mint.key().as_ref(),
        ],
        bump,
        payer = creator,
        space = PoolState::LEN
    )]
    pub pool_state: Box<Account<'info, PoolState>>,  // ← Automatically detected!

    pub amm_config: Box<Account<'info, AmmConfig>>,
    pub token_0_mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_1_mint: Box<InterfaceAccount<'info, Mint>>,
    // ... other fields
}
```

### Step 2: Import and use normally

```rust
// lib.rs
pub use crate::instructions::initialize::Initialize;  // Import your instruction struct
pub use crate::states::PoolState;

#[add_compressible_instructions(PoolState)]  // ← Works automatically now!
#[program]
pub mod raydium_cp_swap {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, ...) -> Result<()> {
        // Your initialization logic
    }

    // ... other instructions
}
```

**That's it!** The macro automatically:

- ✅ Finds the `Initialize` struct with `derive(Compressible)`
- ✅ Extracts the exact seeds from the `#[account(init, seeds = [...], bump)]` attribute
- ✅ Generates compression instructions using those seeds
- ✅ Works with any account types and seed patterns
- ✅ No hardcoded patterns or guessing required

## Multiple Account Types

You can use the same approach for multiple account types:

```rust
// Different instruction structs with different account types
#[derive(Accounts, Compressible)]
pub struct CreateUser<'info> {
    #[account(init, seeds = [b"user", authority.key().as_ref()], bump)]
    pub user_account: Account<'info, UserAccount>,
    pub authority: Signer<'info>,
}

#[derive(Accounts, Compressible)]
pub struct InitializeVault<'info> {
    #[account(init, seeds = [b"vault", mint.key().as_ref()], bump)]
    pub vault: Account<'info, TokenVault>,
    pub mint: Account<'info, Mint>,
}

// All work automatically
#[add_compressible_instructions(PoolState, UserAccount, TokenVault)]
#[program]
pub mod my_program {
    // ...
}
```

## Generated Instructions

For each account type, the macro generates:

- **`compress_{type_name}`** - Compresses the PDA using the exact same seeds
- **`decompress_accounts_idempotent`** - Batch decompress multiple accounts
- **`initialize_compression_config`** - Set up compression configuration
- **`update_compression_config`** - Update compression settings

## Key Benefits of the New Approach

1. **🎯 100% Accurate**: Uses the exact seeds from your instruction structs
2. **🔄 Zero Duplication**: No need to specify seeds twice
3. **🛡️ Type Safe**: Compile-time verification of account types
4. **📁 Multi-File Support**: Works with any project structure
5. **🚀 Future Proof**: Supports any seed patterns, not just common ones
6. **⚡ Automatic**: No configuration or setup required

## Migration from Previous Versions

If you were using the old pattern-matching approach, simply:

1. Add `#[derive(Compressible)]` to your instruction structs
2. Remove any workaround code or manual seed specifications
3. The macro now works automatically!

```diff
// Before (workarounds needed)
- #[add_compressible_instructions(PoolState@[POOL_SEED.as_bytes(), ...])]

// After (completely automatic)
+ #[derive(Accounts, Compressible)]
+ pub struct Initialize<'info> { /* seeds automatically detected */ }
+ #[add_compressible_instructions(PoolState)]
```

## Error Messages

If you forget to add `derive(Compressible)`, you'll get helpful guidance:

```
No seed registry found for type 'PoolState'.

To use this type with #[add_compressible_instructions], you need to:

1. Apply #[derive(Compressible)] to an instruction struct that initializes this account type:

#[derive(Accounts, Compressible)]
pub struct Initialize<'info> {
    #[account(init, seeds = [...], bump)]
    pub pool_state: Account<'info, PoolState>,
}

2. Make sure the instruction struct is imported in the same module where #[add_compressible_instructions] is used:

pub use crate::instructions::initialize::Initialize;
```

This approach completely solves the external file module limitation while being more robust and user-friendly than any pattern matching could be!

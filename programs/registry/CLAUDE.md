# Registry Program - Wrapper Instructions

## Overview

The registry program wraps underlying program instructions (primarily Account Compression) with access control, forester eligibility checks, and work tracking. This pattern ensures decentralized operations are properly authorized and tracked for rewards.

## Core Pattern

Every wrapper instruction:
1. **Loads target account** - deserializes account data to access metadata
2. **Checks forester eligibility** - validates authority and tracks work performed
3. **Executes CPI** - delegates to target program with PDA signer

```rust
pub fn wrapper_instruction<'info>(
    ctx: Context<'_, '_, '_, 'info, WrapperContext<'info>>,
    bump: u8,           // CPI authority PDA bump
    data: Vec<u8>,      // Serialized instruction data to pass through
) -> Result<()> {
    // 1. Load account data (deserialization method depends on account type)
    let account = AccountType::from_account_info(&ctx.accounts.target_account)?;
    
    // 2. Check forester eligibility and track work
    check_forester(
        &account.metadata,
        ctx.accounts.authority.key(),
        ctx.accounts.target_account.key(),
        &mut ctx.accounts.registered_forester_pda,
        work_units,  // Determined by operation type
    )?;
    
    // 3. Delegate to CPI processing function
    process_wrapper_cpi(&ctx, bump, data)
}
```

### Examples of Wrapper Instructions

**Batched operations:**
- `batch_update_address_tree` - Updates multiple addresses in batches
- `batch_append` - Appends batched leaves to output queue
- `batch_nullify` - Nullifies batched leaves from input queue

**Tree management:**
- `rollover_state_merkle_tree_and_queue` - Migrates to new tree when full
- `rollover_batched_address_merkle_tree` - Rolls over batched address tree
- `initialize_batched_state_merkle_tree` - Creates new batched tree

**Single operations:**
- `nullify` - Nullifies individual leaves
- `update_address_merkle_tree` - Updates single address
- `migrate_state` - Migrates leaves between trees

## Account Context

Standard accounts needed for wrapper instructions:

```rust
#[derive(Accounts)]
pub struct WrapperContext<'info> {
    /// Optional: forester PDA for network trees
    #[account(mut)]
    pub registered_forester_pda: Option<Account<'info, ForesterEpochPda>>,
    
    /// Transaction authority
    pub authority: Signer<'info>,
    
    /// PDA that signs CPIs to Account Compression
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority: AccountInfo<'info>,
    
    /// Program access control
    pub registered_program_pda: AccountInfo<'info>,
    
    /// Target program
    pub account_compression_program: Program<'info, AccountCompression>,
    
    /// Event logging
    pub log_wrapper: UncheckedAccount<'info>,
    
    /// Target account being operated on
    #[account(mut)]
    pub target_account: AccountInfo<'info>,
    
    // Additional operation-specific accounts...
}
```

## CPI Processing

The processing function creates the CPI with PDA signer:

```rust
pub fn process_wrapper_cpi(
    ctx: &Context<WrapperContext>,
    bump: u8,
    data: Vec<u8>,
) -> Result<()> {
    // Setup PDA signer seeds
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];
    
    // Prepare CPI accounts (structure matches target program's instruction)
    let accounts = target_program::cpi::accounts::InstructionAccounts {
        authority: ctx.accounts.cpi_authority.to_account_info(),
        target: ctx.accounts.target_account.to_account_info(),
        registered_program_pda: Some(ctx.accounts.registered_program_pda.clone()),
        log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
        // Map remaining accounts from context
    };
    
    // Execute CPI with PDA as signer
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.target_program.to_account_info(),
        accounts,
        signer_seeds,
    );
    
    // Call target program's instruction with data
    target_program::cpi::instruction_name(cpi_ctx, data)
}
```

## Forester Eligibility

The `check_forester` function validates operation authority:

- **With forester PDA**: Validates epoch registration, checks eligibility, tracks work, requires network fee
- **Without forester PDA**: Checks if authority matches tree's designated forester (private trees)

## Adding New Wrapper Instructions

### Step 1: Create Account Context
Create `src/account_compression_cpi/new_operation.rs`:
- Define `NewOperationContext` struct with required accounts
- Import necessary types and constants

### Step 2: Implement CPI Processing
Add `process_new_operation` function:
- Setup PDA signer seeds
- Map accounts to target program's expected structure
- Execute CPI with signer

### Step 3: Add Instruction Handler
In `lib.rs`:
- Load account to get metadata (method varies by account type)
- Determine work units (batch size, DEFAULT_WORK_V1, or custom)
- Call `check_forester` with appropriate parameters
- Call processing function

### Step 4: Export Module
- Add `pub mod new_operation;` to `account_compression_cpi/mod.rs`
- Add `pub use new_operation::*;` export
- Import in `lib.rs` with `use` statement

## Key Implementation Details

### Work Units
- **Batch operations**: Use `account.queue_batches.batch_size`
- **Single operations**: Use `DEFAULT_WORK_V1` constant
- **Custom**: Calculate based on operation complexity

### Account Loading
- **Batched accounts**: `BatchedMerkleTreeAccount::type_from_account_info()`
- **Regular accounts**: `ctx.accounts.account.load()?.metadata`
- **Raw deserialization**: Custom deserialization logic

### Data Parameter
- Contains serialized instruction data for target program
- Passed through unchanged to maintain compatibility
- Target program handles deserialization

### Error Handling
- `InvalidSigner`: Authority not authorized
- `InvalidNetworkFee`: Fee mismatch
- `ForesterDefined/Undefined`: Incorrect forester setup
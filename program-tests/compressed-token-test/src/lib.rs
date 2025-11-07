#![allow(clippy::too_many_arguments)]
#![allow(unexpected_cfgs)]
#![allow(deprecated)]

use anchor_lang::{prelude::*, solana_program::instruction::Instruction};

declare_id!("CompressedTokenTestProgram11111111111111111");

#[program]
pub mod compressed_token_test {
    use super::*;

    /// Wrapper for write_to_cpi_context mode mint_action CPI
    /// All accounts are in remaining_accounts (unchecked)
    pub fn write_to_cpi_context_mint_action<'info>(
        ctx: Context<'_, '_, '_, 'info, MintActionCpiWrapper<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        execute_mint_action_cpi(ctx, inputs)
    }

    /// Wrapper for execute_cpi_context mode mint_action CPI
    /// All accounts are in remaining_accounts (unchecked)
    pub fn execute_cpi_context_mint_action<'info>(
        ctx: Context<'_, '_, '_, 'info, MintActionCpiWrapper<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        execute_mint_action_cpi(ctx, inputs)
    }
}

/// Minimal account structure - only compressed token program ID
/// Everything else goes in remaining_accounts with no validation
#[derive(Accounts)]
pub struct MintActionCpiWrapper<'info> {
    /// CHECK: Compressed token program - no validation
    pub compressed_token_program: AccountInfo<'info>,
}

/// Shared implementation for both wrapper instructions
/// Passes through raw instruction bytes and accounts without any validation
fn execute_mint_action_cpi<'info>(
    ctx: Context<'_, '_, '_, 'info, MintActionCpiWrapper<'info>>,
    inputs: Vec<u8>,
) -> Result<()> {
    // Build account_metas from remaining_accounts - pass through as-is
    let account_metas: Vec<AccountMeta> = ctx
        .remaining_accounts
        .iter()
        .map(|acc| {
            if acc.is_writable {
                AccountMeta::new(*acc.key, acc.is_signer)
            } else {
                AccountMeta::new_readonly(*acc.key, acc.is_signer)
            }
        })
        .collect();

    // Build instruction with raw bytes (no validation)
    let instruction = Instruction {
        program_id: *ctx.accounts.compressed_token_program.key,
        accounts: account_metas,
        data: inputs, // Pass through raw instruction bytes
    };

    // Simple invoke without any signer seeds
    anchor_lang::solana_program::program::invoke(&instruction, ctx.remaining_accounts)?;

    Ok(())
}

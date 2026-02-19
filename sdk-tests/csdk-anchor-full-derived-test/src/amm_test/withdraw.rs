//! Withdraw instruction with BurnCpi.

use anchor_lang::prelude::*;
use light_anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use light_token::instruction::BurnCpi;

use super::states::*;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [AUTH_SEED.as_bytes()],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub pool_state: Box<Account<'info, PoolState>>,

    #[account(mut)]
    pub owner_lp_token: UncheckedAccount<'info>,

    #[account(
        mut,
        token::mint = vault_0_mint,
        token::authority = owner,
    )]
    pub token_0_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = vault_1_mint,
        token::authority = owner,
    )]
    pub token_1_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = token_0_vault.key() == pool_state.token_0_vault,
    )]
    pub token_0_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = token_1_vault.key() == pool_state.token_1_vault,
    )]
    pub token_1_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(address = pool_state.token_0_mint)]
    pub vault_0_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(address = pool_state.token_1_mint)]
    pub vault_1_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        constraint = lp_mint.key() == pool_state.lp_mint,
    )]
    pub lp_mint: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub token_program_2022: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

/// Withdraw instruction handler with BurnCpi.
pub fn process_withdraw(ctx: Context<Withdraw>, lp_token_amount: u64) -> Result<()> {
    // Burn LP tokens from owner using BurnCpi
    BurnCpi {
        source: ctx.accounts.owner_lp_token.to_account_info(),
        mint: ctx.accounts.lp_mint.to_account_info(),
        amount: lp_token_amount,
        authority: ctx.accounts.owner.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        fee_payer: None,
    }
    .invoke()?;

    Ok(())
}

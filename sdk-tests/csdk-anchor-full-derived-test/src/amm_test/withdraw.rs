//! Withdraw instruction with BurnCpi.

use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use light_token_sdk::token::BurnCpi;

use super::states::*;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
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

    pub token_program: Program<'info, Token>,
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
        max_top_up: None,
    }
    .invoke()?;

    Ok(())
}

//! Swap instruction with directional vault aliases.
//!
//! This tests the divergent naming pattern where:
//! - `input_vault` and `output_vault` are aliases for `token_0_vault` / `token_1_vault`
//! - The actual mapping depends on trade direction (ZeroForOne vs OneForZero)
//!
//! Key constraints:
//! - input_vault.key() == pool_state.token_0_vault || input_vault.key() == pool_state.token_1_vault
//! - output_vault.key() == pool_state.token_0_vault || output_vault.key() == pool_state.token_1_vault

use anchor_lang::prelude::*;
use light_anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use super::states::*;

/// Trade direction for swap
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum TradeDirection {
    /// Swap token_0 for token_1
    ZeroForOne,
    /// Swap token_1 for token_0
    OneForZero,
}

#[derive(Accounts)]
pub struct Swap<'info> {
    /// The user performing the swap
    pub payer: Signer<'info>,

    /// CHECK: pool vault and lp mint authority
    #[account(
        seeds = [AUTH_SEED.as_bytes()],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    /// The program account of the pool in which the swap will be performed
    #[account(mut)]
    pub pool_state: Box<Account<'info, PoolState>>,

    /// The user token account for input token
    #[account(mut)]
    pub input_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The user token account for output token
    #[account(mut)]
    pub output_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The vault token account for input token
    /// DIVERGENT NAMING: This is either token_0_vault or token_1_vault depending on direction
    #[account(
        mut,
        constraint = input_vault.key() == pool_state.token_0_vault || input_vault.key() == pool_state.token_1_vault
    )]
    pub input_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The vault token account for output token
    /// DIVERGENT NAMING: This is either token_0_vault or token_1_vault depending on direction
    #[account(
        mut,
        constraint = output_vault.key() == pool_state.token_0_vault || output_vault.key() == pool_state.token_1_vault
    )]
    pub output_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// SPL program for input token transfers
    pub input_token_program: Interface<'info, TokenInterface>,

    /// SPL program for output token transfers
    pub output_token_program: Interface<'info, TokenInterface>,

    /// The mint of input token
    #[account(address = input_vault.mint)]
    pub input_token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// The mint of output token
    #[account(address = output_vault.mint)]
    pub output_token_mint: Box<InterfaceAccount<'info, Mint>>,

    /// The program account for the most recent oracle observation
    #[account(mut, address = pool_state.observation_key)]
    pub observation_state: Box<Account<'info, ObservationState>>,
}

/// Swap instruction handler (noop for testing divergent naming pattern).
///
/// In production, this would:
/// 1. Determine trade direction from input_vault/output_vault
/// 2. Calculate swap amounts
/// 3. Transfer tokens
/// 4. Update observation state
pub fn process_swap(
    _ctx: Context<Swap>,
    _amount_in: u64,
    _minimum_amount_out: u64,
    _direction: TradeDirection,
) -> Result<()> {
    // Noop - just validates accounts can be passed with divergent naming
    Ok(())
}

use anchor_lang::prelude::*;

use crate::cpi_context_account::CpiContextAccount;

pub const CPI_SEED: &[u8] = b"cpi_signature_pda";

#[derive(Accounts)]
pub struct InitializeCpiContextAccount<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(zero)]
    pub cpi_context_account: Account<'info, CpiContextAccount>,
    /// CHECK: manually in instruction
    pub associated_merkle_tree: AccountInfo<'info>,
}

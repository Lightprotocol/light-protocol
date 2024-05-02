use account_compression::StateMerkleTreeAccount;
use anchor_lang::prelude::*;

use super::account::CpiContextAccount;
pub const CPI_SEED: &[u8] = b"cpi_signature_pda";

#[derive(Accounts)]
pub struct InitializeCpiContextAccount<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(zero)]
    pub cpi_context_account: Account<'info, CpiContextAccount>,
    pub system_program: Program<'info, System>,
    pub associated_merkle_tree: AccountLoader<'info, StateMerkleTreeAccount>,
}

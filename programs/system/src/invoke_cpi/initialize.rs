use account_compression::StateMerkleTreeAccount;
use anchor_lang::prelude::*;

use super::account::CpiContextAccount;
pub const CPI_SEED: &[u8] = b"cpi_signature_pda";

// TODO: add support for batched Merkle trees by manually deserializing the
// associated_merkle_tree account
#[derive(Accounts)]
pub struct InitializeCpiContextAccount<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(zero)]
    pub cpi_context_account: Account<'info, CpiContextAccount>,
    pub associated_merkle_tree: AccountLoader<'info, StateMerkleTreeAccount>,
}

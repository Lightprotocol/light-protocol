use anchor_lang::prelude::*;

use crate::{utils::constants::STORAGE_SEED, MerkleTreeUpdateState};

#[derive(Accounts)]
pub struct CloseUpdateState<'info> {
    #[account(mut, address=merkle_tree_update_state.load()?.relayer)]
    pub authority: Signer<'info>,
    /// CHECK:` Is the merkle_tree_update_state of an authority.
    #[account(
        mut,
        seeds = [authority.key().to_bytes().as_ref(), STORAGE_SEED],
        bump,
        close=authority
    )]
    pub merkle_tree_update_state: AccountLoader<'info, MerkleTreeUpdateState>,
}

#[cfg(feature = "atomic-transactions")]
pub fn process_close_merkle_tree_update_state() -> Result<()> {
    use crate::errors::ErrorCode;
    err!(ErrorCode::AtomicTransactionsEnabled)
}

#[cfg(not(feature = "atomic-transactions"))]
pub fn process_close_merkle_tree_update_state() -> Result<()> {
    Ok(())
}

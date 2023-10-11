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

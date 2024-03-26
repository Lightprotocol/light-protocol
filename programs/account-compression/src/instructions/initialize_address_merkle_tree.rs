pub use anchor_lang::prelude::*;

use crate::{
    address_mt_from_bytes_zero_copy_init, errors::AccountCompressionErrorCode,
    state::AddressMerkleTreeAccount,
};

#[derive(Accounts)]
pub struct InitializeAddressMerkleTree<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(zero)]
    pub merkle_tree: AccountLoader<'info, AddressMerkleTreeAccount>,
}

pub fn process_initialize_address_merkle_tree<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeAddressMerkleTree<'info>>,
    index: u64,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    height: u64,
    changelog_size: u64,
    roots_size: u64,
    canopy_depth: u64,
) -> Result<()> {
    let mut address_merkle_tree = ctx.accounts.merkle_tree.load_init()?;

    address_merkle_tree.index = index;
    address_merkle_tree.owner = owner;
    address_merkle_tree.delegate = delegate.unwrap_or(owner);

    address_mt_from_bytes_zero_copy_init(
        ctx.accounts.merkle_tree,
        height as usize,
        changelog_size as usize,
        roots_size as usize,
        canopy_depth as usize,
    )
    .map_err(ProgramError::from)?;

    Ok(())
}

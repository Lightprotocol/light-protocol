use account_compression_state::address_merkle_tree_from_bytes_mut;
pub use anchor_lang::prelude::*;

use crate::{errors::AccountCompressionErrorCode, state::AddressMerkleTreeAccount};

#[derive(Accounts)]
pub struct InitializeAddressMerkleTree<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(zero)]
    pub merkle_tree: AccountLoader<'info, AddressMerkleTreeAccount>,
}

pub fn process_initialize_address_merkle_tree<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeAddressMerkleTree<'info>>,
) -> Result<()> {
    let mut address_merkle_tree = ctx.accounts.merkle_tree.load_init()?;
    let address_merkle_tree =
        address_merkle_tree_from_bytes_mut(&mut address_merkle_tree.merkle_tree);
    address_merkle_tree
        .init()
        .map_err(|_| AccountCompressionErrorCode::AddressMerkleTreeInitialize)?;

    Ok(())
}

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
    index: u64,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    height: u64,
    changelog_size: u64,
    roots_size: u64,
) -> Result<()> {
    let mut address_merkle_tree = ctx.accounts.merkle_tree.load_init()?;

    address_merkle_tree.index = index;
    address_merkle_tree.owner = owner;
    address_merkle_tree.delegate = delegate.unwrap_or(owner);

    address_merkle_tree
        .load_merkle_tree_init(
            height
                .try_into()
                .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?,
            changelog_size
                .try_into()
                .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?,
            roots_size
                .try_into()
                .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?,
        )
        .map_err(ProgramError::from)?;

    Ok(())
}

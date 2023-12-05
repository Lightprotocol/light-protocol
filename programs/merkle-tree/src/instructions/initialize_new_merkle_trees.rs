use anchor_lang::prelude::*;

use crate::{
    errors::ErrorCode,
    event_merkle_tree::EventMerkleTree,
    process_initialize_new_event_merkle_tree, process_initialize_new_merkle_tree,
    transaction_merkle_tree::state::TransactionMerkleTree,
    utils::{
        accounts::deserialize_and_update_old_merkle_tree,
        config::MERKLE_TREE_HEIGHT,
        constants::{
            EVENT_MERKLE_TREE_SEED, MERKLE_TREE_AUTHORITY_SEED, TRANSACTION_MERKLE_TREE_SEED,
        },
    },
    MerkleTreeAuthority,
};

#[derive(Accounts)]
pub struct InitializeNewMerkleTrees<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        seeds = [
            TRANSACTION_MERKLE_TREE_SEED,
            merkle_tree_authority_pda.transaction_merkle_tree_index.to_le_bytes().as_ref(),
        ],
        bump,
        payer = authority,
        space = TransactionMerkleTree::LEN,
    )]
    pub new_transaction_merkle_tree: AccountLoader<'info, TransactionMerkleTree>,
    #[account(
        init,
        seeds = [
            EVENT_MERKLE_TREE_SEED,
            merkle_tree_authority_pda.event_merkle_tree_index.to_le_bytes().as_ref(),
        ],
        bump,
        payer = authority,
        space = EventMerkleTree::LEN,
    )]
    pub new_event_merkle_tree: AccountLoader<'info, EventMerkleTree>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    #[account(mut, seeds = [MERKLE_TREE_AUTHORITY_SEED], bump)]
    pub merkle_tree_authority_pda: Account<'info, MerkleTreeAuthority>,
}

#[allow(unused_variables)]
pub fn process_initialize_new_merkle_trees(
    ctx: Context<InitializeNewMerkleTrees>,
    lock_duration: u64,
) -> Result<()> {
    if !ctx
        .accounts
        .merkle_tree_authority_pda
        .enable_permissionless_merkle_tree_registration
        && ctx.accounts.authority.key() != ctx.accounts.merkle_tree_authority_pda.pubkey
    {
        return err!(ErrorCode::InvalidAuthority);
    }

    if ctx.remaining_accounts.len() != 2 {
        return err!(ErrorCode::ExpectedOldMerkleTrees);
    }

    let merkle_tree_authority = &mut ctx.accounts.merkle_tree_authority_pda;

    // Transaction Merkle Tree
    deserialize_and_update_old_merkle_tree::<TransactionMerkleTree>(
        &ctx.remaining_accounts[0],
        TRANSACTION_MERKLE_TREE_SEED,
        ctx.program_id,
    )?;
    let new_transaction_merkle_tree = &mut ctx.accounts.new_transaction_merkle_tree.load_init()?;
    process_initialize_new_merkle_tree(
        new_transaction_merkle_tree,
        merkle_tree_authority,
        MERKLE_TREE_HEIGHT,
    )?;

    // Event Merkle Tree
    deserialize_and_update_old_merkle_tree::<EventMerkleTree>(
        &ctx.remaining_accounts[1],
        EVENT_MERKLE_TREE_SEED,
        ctx.program_id,
    )?;
    let new_event_merkle_tree = &mut ctx.accounts.new_event_merkle_tree.load_init()?;
    process_initialize_new_event_merkle_tree(new_event_merkle_tree, merkle_tree_authority)?;

    #[cfg(not(feature = "atomic-transactions"))]
    crate::process_update_lock_duration(new_transaction_merkle_tree, lock_duration)?;

    Ok(())
}

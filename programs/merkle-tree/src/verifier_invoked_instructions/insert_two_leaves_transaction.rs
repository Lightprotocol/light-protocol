use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{
    emit_indexer_event, state::MerkleTreeSet, state_merkle_tree_from_bytes_mut, ChangelogEvent,
    ChangelogEventV1, RegisteredVerifier,
};

#[derive(Accounts)]
pub struct InsertTwoLeaves<'info> {
    /// CHECK: should only be accessed by a registered verifier.
    #[account(
        mut,
        seeds=[__program_id.to_bytes().as_ref()],
        bump,
        seeds::program=registered_verifier_pda.pubkey
    )]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub merkle_tree_set: AccountLoader<'info, MerkleTreeSet>,
    pub system_program: Program<'info, System>,
    #[account(seeds=[&registered_verifier_pda.pubkey.to_bytes()],  bump)]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>,
    /// CHECK: It's checked in `emit_indexer_event`.
    pub log_wrapper: UncheckedAccount<'info>,
}

pub fn process_insert_two_leaves<'info, 'a>(
    ctx: Context<'a, '_, '_, 'info, InsertTwoLeaves<'info>>,
    leaves: &'a [[u8; 32]],
) -> Result<()> {
    let mut merkle_tree_set = ctx.accounts.merkle_tree_set.load_mut()?;
    // Borrow `state_merkle_tree` mutably.
    let state_merkle_tree =
        state_merkle_tree_from_bytes_mut(&mut merkle_tree_set.state_merkle_tree);

    // Iterate over the leaves in pairs
    for i in (0..leaves.len()).step_by(2) {
        // Get the left leaf
        let leaf_left = &leaves[i];

        // Check whether there is a right leaf; return an error if not
        let leaf_right = if i + 1 < leaves.len() {
            &leaves[i + 1]
        } else {
            return err!(crate::errors::ErrorCode::OddNumberOfLeaves);
        };

        // Insert the pair into the merkle tree
        let changelog_entries = state_merkle_tree
            .append_batch(&[leaf_left, leaf_right])
            .map_err(ProgramError::from)?;

        let changelog_event = ChangelogEvent::V1(ChangelogEventV1::new(
            ctx.accounts.merkle_tree_set.key(),
            &changelog_entries,
            state_merkle_tree.sequence_number,
        )?);
        emit_indexer_event(
            changelog_event.try_to_vec()?,
            &ctx.accounts.log_wrapper,
            &ctx.accounts.authority,
        )?;
    }

    Ok(())
}

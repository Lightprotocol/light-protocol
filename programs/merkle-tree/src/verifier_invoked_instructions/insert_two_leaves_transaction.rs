use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{state::MerkleTreeSet, state_merkle_tree_from_bytes_mut, RegisteredVerifier};

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
}

pub fn process_insert_two_leaves<'info, 'a>(
    ctx: Context<'a, '_, '_, 'info, InsertTwoLeaves<'info>>,
    leaves: &'a [[u8; 32]],
) -> Result<()> {
    let mut merkle_tree_set = ctx.accounts.merkle_tree_set.load_mut()?;

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
        state_merkle_tree_from_bytes_mut(&mut merkle_tree_set.state_merkle_tree)
            .append_two(leaf_left, leaf_right)?;
    }

    Ok(())
}

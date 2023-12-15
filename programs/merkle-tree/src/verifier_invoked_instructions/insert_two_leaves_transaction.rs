use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};

use crate::{
    transaction_merkle_tree::state::TransactionMerkleTree,
    utils::constants::TRANSACTION_MERKLE_TREE_SEED, RegisteredVerifier,
};

#[derive(Accounts)]
pub struct InsertTwoLeaves<'info> {
    /// CHECK: should only be accessed by a registered verifier.
    #[account(mut, seeds=[__program_id.to_bytes().as_ref()],bump,seeds::program=registered_verifier_pda.pubkey)]
    pub authority: Signer<'info>,
    #[account(mut, seeds = [
        TRANSACTION_MERKLE_TREE_SEED,
        transaction_merkle_tree.load().unwrap().merkle_tree_nr.to_le_bytes().as_ref()
    ], bump)]
    pub transaction_merkle_tree: AccountLoader<'info, TransactionMerkleTree>,
    pub system_program: Program<'info, System>,
    #[account(seeds=[&registered_verifier_pda.pubkey.to_bytes()],  bump)]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>,
}

pub fn process_insert_two_leaves<'info, 'a>(
    ctx: Context<'a, '_, '_, 'info, InsertTwoLeaves<'info>>,
    leaves: &'a Vec<[u8; 32]>,
) -> Result<()> {
    let merkle_tree = &mut ctx.accounts.transaction_merkle_tree.load_mut()?;

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
        merkle_tree.merkle_tree.insert(*leaf_left, *leaf_right)?;

        // Increase next index by 2 because we're inserting 2 leaves at once
        merkle_tree.next_queued_index += 2;
    }

    Ok(())
}

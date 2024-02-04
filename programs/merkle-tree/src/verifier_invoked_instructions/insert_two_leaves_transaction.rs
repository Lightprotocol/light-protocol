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
        let changelog_entries = state_merkle_tree.append_batch(&[leaf_left, leaf_right])?;

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

// #[derive(Accounts)]
// pub struct InsertTwoLeavesParallel<'info> {
//     /// CHECK: should only be accessed by a registered verifier.
//     #[account(mut, seeds=[__program_id.to_bytes().as_ref()],bump,seeds::program=registered_verifier_pda.pubkey)]
//     pub authority: Signer<'info>,
//     #[account(seeds=[&registered_verifier_pda.pubkey.to_bytes()],  bump)]
//     pub registered_verifier_pda: Account<'info, RegisteredVerifier>,
// }

// // every leaf could be inserted into a different Merkle tree account
// // deduplicate Merkle trees and identify into which tree to insert what leaf
// pub fn process_insert_two_leaves_parallel<'info, 'a>(
//     ctx: Context<'a, '_, '_, 'info, InsertTwoLeavesParallel<'info>>,
//     leaves: &'a [[u8; 32]],
// ) -> Result<()> {
//     let mut merkle_tree_map = HashMap::<Pubkey, (&AccountInfo, Vec<[u8; 32]>)>::new();
//     for (i, mt) in ctx.remaining_accounts.iter().enumerate() {
//         match merkle_tree_map.get(&mt.key()) {
//             Some(_) => {}
//             None => {
//                 merkle_tree_map.insert(mt.key(), (mt, Vec::new()));
//             }
//         };
//         merkle_tree_map
//             .get_mut(&mt.key())
//             .unwrap()
//             .1
//             .push(leaves[i]);
//     }

//     for (mt, leaves) in merkle_tree_map.values() {
//         let merkle_tree = AccountLoader::<TransactionMerkleTree>::try_from(mt).unwrap();
//         let mut merkle_tree = merkle_tree.load_mut()?;
//         for leaf in leaves.chunks(2) {
//             // TODO: allow single leaf insertions after rebasing
//             if leaf.len() != 2 {
//                 return err!(crate::errors::ErrorCode::OddNumberOfLeaves);
//             };
//             merkle_tree.merkle_tree.insert(leaf[0], leaf[1])?;
//         }
//     }

//     Ok(())
// }

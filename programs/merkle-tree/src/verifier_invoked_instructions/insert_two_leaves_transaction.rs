use anchor_lang::prelude::*;

use crate::{
    transaction_merkle_tree::state::{TransactionMerkleTree, TwoLeavesBytesPda},
    utils::constants::{LEAVES_SEED, TRANSACTION_MERKLE_TREE_SEED},
    RegisteredVerifier,
};

#[derive(Accounts)]
#[instruction(
    leaf_left: [u8;32],
    leaf_right: [u8;32],
    encrypted_utxos: [u8;256],
)]
pub struct InsertTwoLeaves<'info> {
    /// CHECK:` should only be accessed by a registered verifier.
    #[account(mut, seeds=[__program_id.to_bytes().as_ref()],bump,seeds::program=registered_verifier_pda.pubkey)]
    pub authority: Signer<'info>,
    // /// CHECK:` Leaves account should be checked by invoking verifier.
    #[account(init, seeds= [&leaf_left, LEAVES_SEED], bump, payer=authority, space= 8 + 3 * 32 + 256 + 8 + 8)]
    pub two_leaves_pda: Account<'info, TwoLeavesBytesPda>,
    #[account(mut, seeds = [
        TRANSACTION_MERKLE_TREE_SEED,
        transaction_merkle_tree.load().unwrap().merkle_tree_nr.to_le_bytes().as_ref()
    ], bump)]
    pub transaction_merkle_tree: AccountLoader<'info, TransactionMerkleTree>,
    pub system_program: Program<'info, System>,
    #[account(seeds=[&registered_verifier_pda.pubkey.to_bytes()],  bump)]
    pub registered_verifier_pda: Account<'info, RegisteredVerifier>,
}

pub fn process_insert_two_leaves(
    ctx: Context<InsertTwoLeaves>,
    leaf_left: [u8; 32],
    leaf_right: [u8; 32],
    encrypted_utxos: [u8; 256],
) -> Result<()> {
    //save leaves into pda account
    ctx.accounts.two_leaves_pda.node_left = leaf_left;
    ctx.accounts.two_leaves_pda.node_right = leaf_right;
    let mut merkle_tree = ctx.accounts.transaction_merkle_tree.load_mut()?;
    ctx.accounts.two_leaves_pda.left_leaf_index = merkle_tree.next_queued_index;

    ctx.accounts.two_leaves_pda.merkle_tree_pubkey = ctx.accounts.transaction_merkle_tree.key();
    ctx.accounts.two_leaves_pda.encrypted_utxos = encrypted_utxos;

    // Increase next index by 2 because we're inserting 2 leaves at once.
    merkle_tree.next_queued_index += 2;
    Ok(())
}

use anchor_lang::prelude::*;
use light_concurrent_merkle_tree::event::{IndexedMerkleTreeEvent, MerkleTreeEvent};
use light_indexed_merkle_tree::array::IndexedElement;
use num_bigint::BigUint;

use crate::{
    emit_indexer_event,
    errors::AccountCompressionErrorCode,
    from_vec,
    state::{queue_from_bytes_zero_copy_mut, QueueAccount},
    AddressMerkleTreeAccount,
};

#[derive(Accounts)]
pub struct UpdateMerkleTree<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub queue: AccountLoader<'info, QueueAccount>,
    #[account(mut)]
    pub merkle_tree: AccountLoader<'info, AddressMerkleTreeAccount>,
    /// CHECK: in event emitting
    pub log_wrapper: UncheckedAccount<'info>,
}

#[allow(clippy::too_many_arguments)]
pub fn process_update_address_merkle_tree<'info>(
    ctx: Context<'_, '_, '_, 'info, UpdateMerkleTree<'info>>,
    // Index of the Merkle tree changelog.
    changelog_index: u16,
    // Address to dequeue.
    value_index: u16,
    // Low address.
    low_address_index: u64,
    low_address_value: [u8; 32],
    low_address_next_index: u64,
    // Value of the next address.
    low_address_next_value: [u8; 32],
    // Merkle proof for updating the low address.
    low_address_proof: [[u8; 32]; 16],
) -> Result<()> {
    let address_queue = ctx.accounts.queue.to_account_info();
    let mut address_queue = address_queue.try_borrow_mut_data()?;
    let mut address_queue = unsafe { queue_from_bytes_zero_copy_mut(&mut address_queue)? };

    let mut merkle_tree = ctx.accounts.merkle_tree.load_mut()?;
    if merkle_tree.metadata.associated_queue != ctx.accounts.queue.key() {
        msg!(
            "Merkle tree and nullifier queue are not associated. Merkle tree associated nullifier queue {} != nullifier queue {}",
            merkle_tree.metadata.associated_queue,
            ctx.accounts.queue.key()
        );
        return err!(AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated);
    }
    let merkle_tree = merkle_tree.load_merkle_tree_mut()?;

    let sequence_number = merkle_tree.merkle_tree.merkle_tree.sequence_number;

    let value = address_queue
        .get_unmarked_bucket(value_index as usize)
        .ok_or(AccountCompressionErrorCode::LeafNotFound)?
        .ok_or(AccountCompressionErrorCode::LeafNotFound)?
        .value_biguint();

    // Update the address with ranges adjusted to the Merkle tree state.
    let address: IndexedElement<usize> = IndexedElement {
        index: merkle_tree.merkle_tree.merkle_tree.next_index,
        value: value.clone(),
        next_index: low_address_next_index as usize,
    };

    // Convert byte inputs to big integers.
    let low_address: IndexedElement<usize> = IndexedElement {
        index: low_address_index as usize,
        value: BigUint::from_bytes_be(&low_address_value),
        next_index: low_address_next_index as usize,
    };

    let low_address_next_value = BigUint::from_bytes_be(&low_address_next_value);

    let mut proof = from_vec(
        low_address_proof.as_slice(),
        merkle_tree.merkle_tree.merkle_tree.height,
    )
    .map_err(ProgramError::from)?;
    // Update the Merkle tree.
    // Inputs check:
    // - changelog index gets values from account
    // - address is selected by value index from hashset
    // - low address and low address next value are validated with low address Merkle proof
    let indexed_merkle_tree_update = merkle_tree
        .merkle_tree
        .update(
            usize::from(changelog_index),
            address,
            low_address,
            low_address_next_value,
            &mut proof,
        )
        .map_err(ProgramError::from)?;

    // Mark the address with the current sequence number.
    address_queue
        .mark_with_sequence_number(&value, sequence_number)
        .map_err(ProgramError::from)?;

    let address_event = MerkleTreeEvent::V3(IndexedMerkleTreeEvent {
        id: ctx.accounts.merkle_tree.key().to_bytes(),
        updates: vec![indexed_merkle_tree_update],
        // Address Merkle tree update does one update and one append,
        // thus the first seq number is final seq - 1.
        seq: merkle_tree.merkle_tree.merkle_tree.sequence_number as u64 - 1,
    });
    emit_indexer_event(
        address_event.try_to_vec()?,
        &ctx.accounts.log_wrapper.to_account_info(),
    )?;
    Ok(())
}

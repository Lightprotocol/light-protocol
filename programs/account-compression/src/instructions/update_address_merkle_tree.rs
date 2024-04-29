use anchor_lang::prelude::*;
use light_bounded_vec::BoundedVec;
use light_indexed_merkle_tree::array::IndexedElement;
use num_bigint::BigUint;

use crate::{
    address_queue_from_bytes_zero_copy_mut,
    state::address::{AddressMerkleTreeAccount, AddressQueueAccount},
};

#[derive(Accounts)]
pub struct UpdateMerkleTree<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub queue: AccountLoader<'info, AddressQueueAccount>,
    #[account(mut)]
    pub merkle_tree: AccountLoader<'info, AddressMerkleTreeAccount>,
}

#[allow(clippy::too_many_arguments)]
pub fn process_update_address_merkle_tree<'info>(
    ctx: Context<'_, '_, '_, 'info, UpdateMerkleTree<'info>>,
    // Index of the Merkle tree changelog.
    changelog_index: u16,
    // Address to dequeue.
    value_index: u16,
    // Index of the next address.
    next_index: usize,
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
    let mut address_queue = unsafe { address_queue_from_bytes_zero_copy_mut(&mut address_queue)? };

    let mut merkle_tree = ctx.accounts.merkle_tree.load_mut()?;
    let merkle_tree = merkle_tree.load_merkle_tree_mut()?;

    let sequence_number = merkle_tree.merkle_tree.sequence_number;

    let value = address_queue
        .by_value_index(value_index as usize, None)
        .unwrap()
        .value_biguint();

    // Update the address with ranges adjusted to the Merkle tree state.
    let address: IndexedElement<usize> = IndexedElement {
        index: merkle_tree.merkle_tree.next_index,
        value: value.clone(),
        next_index,
    };

    // Convert byte inputs to big integers.
    let low_address: IndexedElement<usize> = IndexedElement {
        index: low_address_index as usize,
        value: BigUint::from_bytes_be(&low_address_value),
        next_index: low_address_next_index as usize,
    };
    let low_address_next_value = BigUint::from_bytes_be(&low_address_next_value);

    let mut bounded_vec = BoundedVec::with_capacity(26);
    for element in low_address_proof {
        bounded_vec.push(element).map_err(ProgramError::from)?;
    }
    // Update the Merkle tree.
    // Inputs check:
    // - changelog index gets values from account
    // - address is selected by value index from hashset
    // - low address and low address next value are validated with low address Merkle proof
    merkle_tree
        .update(
            usize::from(changelog_index),
            address,
            low_address,
            low_address_next_value,
            &mut bounded_vec,
        )
        .map_err(ProgramError::from)?;

    // Mark the address with the current sequence number.
    // TODO: replace with root history sequence number
    address_queue
        .mark_with_sequence_number(&value, sequence_number)
        .map_err(ProgramError::from)?;

    Ok(())
}

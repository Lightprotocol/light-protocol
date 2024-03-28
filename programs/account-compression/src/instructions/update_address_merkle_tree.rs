use anchor_lang::prelude::*;
use light_bounded_vec::BoundedVec;
use light_indexed_merkle_tree::array::{IndexedElement, RawIndexedElement};
use num_bigint::BigUint;

use crate::{
    address_queue_from_bytes_zero_copy_mut,
    errors::AccountCompressionErrorCode,
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
    value: [u8; 32],
    // Index of the next address.
    next_index: usize,
    // Value of the next address.
    next_value: [u8; 32],
    // Low address.
    low_address: RawIndexedElement<usize, 32>,
    // Value of the next address.
    low_address_next_value: [u8; 32],
    // Merkle proof for updating the low address.
    low_address_proof: [[u8; 32]; 16],
    // ZK proof for integrity of provided `address_next_index` and
    // `address_next_value`.
    _next_address_proof: [u8; 128],
) -> Result<()> {
    // let address_queue_acc = ctx.accounts.queue.to_account_info();
    // // TODO: check discriminator
    // let address_queue = &mut address_queue_acc.data.borrow_mut()[8..];
    // let address_queue = unsafe { HashSet::<u16>::from_bytes(address_queue) };
    let address_queue = unsafe {
        address_queue_from_bytes_zero_copy_mut(
            ctx.accounts.queue.to_account_info().try_borrow_mut_data()?,
        )?
    };

    let mut merkle_tree = ctx.accounts.merkle_tree.load_mut()?;

    let sequence_number = merkle_tree.load_merkle_tree()?.merkle_tree.sequence_number;
    let value = BigUint::from_bytes_le(value.as_slice());

    // Mark the address with the current sequence number.
    msg!("ayy lmao");
    address_queue
        .mark_with_sequence_number(&value, sequence_number)
        .map_err(ProgramError::from)?;
    msg!("nope");

    // Update the address with ranges adjusted to the Merkle tree state.
    let address: IndexedElement<usize> = IndexedElement {
        index: merkle_tree.load_merkle_tree()?.merkle_tree.next_index,
        value,
        next_index,
    };

    // Convert byte inputs to big integers.
    let next_value = BigUint::from_bytes_le(&next_value);
    let low_address: IndexedElement<usize> = low_address.into();
    let low_address_next_value = BigUint::from_bytes_le(&low_address_next_value);

    // Update the Merkle tree.
    merkle_tree
        .load_merkle_tree_mut()?
        .update(
            usize::from(changelog_index),
            address,
            &next_value,
            low_address,
            &low_address_next_value,
            &mut BoundedVec::from_array(&low_address_proof),
        )
        .map_err(|_| AccountCompressionErrorCode::AddressMerkleTreeUpdate)?;

    Ok(())
}

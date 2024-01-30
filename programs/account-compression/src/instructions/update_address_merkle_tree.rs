use account_compression_state::{address_merkle_tree_from_bytes_mut, address_queue_from_bytes_mut};
use anchor_lang::prelude::*;
use ark_ff::BigInteger256;
use light_indexed_merkle_tree::array::{IndexingElement, RawIndexingElement};
use light_utils::be_bytes_to_bigint;

use crate::{
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
    // Index of the address to dequeue.
    queue_index: u16,
    // Index of the next address.
    address_next_index: u16,
    // Value of the next address.
    address_next_value: [u8; 32],
    // Low address.
    low_address: RawIndexingElement<32>,
    // Value of the next address.
    low_address_next_value: [u8; 32],
    // Merkle proof for updating the low address.
    low_address_proof: [[u8; 32]; 22],
    // ZK proof for integrity of provided `address_next_index` and
    // `address_next_value`.
    _next_address_proof: [u8; 128],
) -> Result<()> {
    let mut address_queue = ctx.accounts.queue.load_mut()?;
    let address_queue = address_queue_from_bytes_mut(&mut address_queue.queue);
    let mut merkle_tree = ctx.accounts.merkle_tree.load_mut()?;
    let merkle_tree = address_merkle_tree_from_bytes_mut(&mut merkle_tree.merkle_tree);

    // Remove the address from the queue.
    let mut address = address_queue
        .dequeue_at(queue_index)
        .map_err(|_| AccountCompressionErrorCode::AddressQueueDequeue)?
        .ok_or(AccountCompressionErrorCode::InvalidIndex)?;

    // Update the address with ranges adjusted to the Merkle tree state.
    address.index = u16::try_from(merkle_tree.merkle_tree.next_index)
        .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?;
    address.next_index = address_next_index;

    // Convert byte inputs to big integers.
    let address_next_value = be_bytes_to_bigint(&address_next_value)
        .map_err(|_| AccountCompressionErrorCode::BytesToBigint)?;
    let low_address: IndexingElement<BigInteger256> = low_address
        .try_into()
        .map_err(|_| AccountCompressionErrorCode::BytesToBigint)?;
    let low_address_next_value = be_bytes_to_bigint(&low_address_next_value)
        .map_err(|_| AccountCompressionErrorCode::BytesToBigint)?;

    // Update the Merkle tree.
    merkle_tree
        .update(
            usize::from(changelog_index),
            address,
            address_next_value,
            low_address,
            low_address_next_value,
            &low_address_proof,
        )
        .map_err(|_| AccountCompressionErrorCode::AddressMerkleTreeUpdate)?;

    Ok(())
}

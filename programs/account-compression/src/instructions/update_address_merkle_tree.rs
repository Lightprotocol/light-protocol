use anchor_lang::prelude::*;
use light_concurrent_merkle_tree::event::{IndexedMerkleTreeEvent, MerkleTreeEvent};
use light_indexed_merkle_tree::array::IndexedElement;
use num_bigint::BigUint;

use crate::{
    address_merkle_tree_from_bytes_zero_copy_mut, emit_indexer_event,
    errors::AccountCompressionErrorCode,
    from_vec,
    state::{queue_from_bytes_zero_copy_mut, QueueAccount},
    utils::check_signer_is_registered_or_authority::{
        check_signer_is_registered_or_authority, GroupAccounts,
    },
    AddressMerkleTreeAccount, RegisteredProgram,
};

#[derive(Accounts)]
pub struct UpdateAddressMerkleTree<'info> {
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    #[account(mut)]
    pub queue: AccountLoader<'info, QueueAccount>,
    #[account(mut)]
    pub merkle_tree: AccountLoader<'info, AddressMerkleTreeAccount>,
    /// CHECK: when emitting event.
    pub log_wrapper: UncheckedAccount<'info>,
}

impl<'info> GroupAccounts<'info> for UpdateAddressMerkleTree<'info> {
    fn get_authority(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

#[allow(clippy::too_many_arguments)]
pub fn process_update_address_merkle_tree<'info>(
    ctx: Context<'_, '_, '_, 'info, UpdateAddressMerkleTree<'info>>,
    // Index of the Merkle tree changelog.
    changelog_index: u16,
    indexed_changelog_index: u16,
    // Address to dequeue.
    value_index: u16,
    // Low address.
    low_address_value: [u8; 32],       // included in leaf hash
    low_address_next_index: u64,       // included in leaf hash
    low_address_next_value: [u8; 32],  // included in leaf hash
    low_address_index: u64,            // leaf index of low element
    low_address_proof: [[u8; 32]; 16], // Merkle proof for updating the low address.
) -> Result<()> {
    let address_queue = ctx.accounts.queue.to_account_info();
    let mut address_queue = address_queue.try_borrow_mut_data()?;
    let mut address_queue = unsafe { queue_from_bytes_zero_copy_mut(&mut address_queue)? };

    {
        let merkle_tree = ctx.accounts.merkle_tree.load_mut()?;
        if merkle_tree.metadata.associated_queue != ctx.accounts.queue.key() {
            msg!(
            "Merkle tree and nullifier queue are not associated. Merkle tree associated address queue {:?} != provided queue {}",
            merkle_tree.metadata.associated_queue,
            ctx.accounts.queue.key()
        );
            return err!(AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated);
        }
        check_signer_is_registered_or_authority::<UpdateAddressMerkleTree, AddressMerkleTreeAccount>(
            &ctx,
            &merkle_tree,
        )?;
    }

    let merkle_tree = ctx.accounts.merkle_tree.to_account_info();
    let mut merkle_tree = merkle_tree.try_borrow_mut_data()?;
    let mut merkle_tree = address_merkle_tree_from_bytes_zero_copy_mut(&mut merkle_tree)?;

    let value = address_queue
        .get_unmarked_bucket(value_index as usize)
        .ok_or(AccountCompressionErrorCode::LeafNotFound)?
        .ok_or(AccountCompressionErrorCode::LeafNotFound)?
        .value_biguint();

    // Indexed Merkle tree update:
    // - the range represented by the low element is split into two ranges
    // - the new low element(lower range, next value is address) and the address
    //   element (higher range, next value is low_element.next_value)
    // - the new low element is updated, and the address element is appended

    // Lower range
    let low_address: IndexedElement<usize> = IndexedElement {
        index: low_address_index as usize,
        value: BigUint::from_bytes_be(&low_address_value),
        next_index: low_address_next_index as usize,
    };

    let low_address_next_value = BigUint::from_bytes_be(&low_address_next_value);

    let mut proof =
        from_vec(low_address_proof.as_slice(), merkle_tree.height).map_err(ProgramError::from)?;

    // Update the Merkle tree.
    // Inputs check:
    // - address is element of (value, next_value)
    // - changelog index gets values from account
    // - indexed changelog index gets values from account
    // - address is selected by value index from hashset
    // - low address and low address next value are validated with low address Merkle proof
    let indexed_merkle_tree_update = merkle_tree
        .update(
            usize::from(changelog_index),
            usize::from(indexed_changelog_index),
            value.clone(),
            low_address,
            low_address_next_value,
            &mut proof,
        )
        .map_err(ProgramError::from)?;

    // Mark the address with the current sequence number.
    address_queue
        .mark_with_sequence_number(value_index as usize, merkle_tree.sequence_number())
        .map_err(ProgramError::from)?;

    let address_event = MerkleTreeEvent::V3(IndexedMerkleTreeEvent {
        id: ctx.accounts.merkle_tree.key().to_bytes(),
        updates: vec![indexed_merkle_tree_update],
        // Address Merkle tree update does one update and one append,
        // thus the first seq number is final seq - 1.
        seq: merkle_tree.sequence_number() as u64 - 1,
    });
    emit_indexer_event(
        address_event.try_to_vec()?,
        &ctx.accounts.log_wrapper.to_account_info(),
    )
}

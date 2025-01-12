pub use anchor_lang::prelude::*;
use light_merkle_tree_metadata::{access::AccessMetadata, rollover::RolloverMetadata};

use crate::{address_merkle_tree_from_bytes_zero_copy_init, state::AddressMerkleTreeAccount};

pub fn process_initialize_address_merkle_tree(
    address_merkle_tree_loader: &AccountLoader<'_, AddressMerkleTreeAccount>,
    index: u64,
    owner: Pubkey,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
    height: u32,
    changelog_size: u64,
    roots_size: u64,
    canopy_depth: u64,
    address_changelog_size: u64,
    associated_queue: Pubkey,
    network_fee: u64,
    rollover_threshold: Option<u64>,
    close_threshold: Option<u64>,
) -> Result<()> {
    {
        let mut merkle_tree = address_merkle_tree_loader.load_init()?;

        // The address Merkle tree is never directly called by the user.
        // All rollover fees are collected by the address queue.
        let rollover_fee = 0;
        merkle_tree.init(
            AccessMetadata {
                owner: owner.into(),
                program_owner: program_owner.unwrap_or_default().into(),
                forester: forester.unwrap_or_default().into(),
            },
            RolloverMetadata::new(
                index,
                rollover_fee,
                rollover_threshold,
                network_fee,
                close_threshold,
                None,
            ),
            associated_queue,
        );
    }

    let merkle_tree = address_merkle_tree_loader.to_account_info();
    let mut merkle_tree = merkle_tree.try_borrow_mut_data()?;
    let mut merkle_tree = address_merkle_tree_from_bytes_zero_copy_init(
        &mut merkle_tree,
        height as usize,
        canopy_depth as usize,
        changelog_size as usize,
        roots_size as usize,
        address_changelog_size as usize,
    )?;
    msg!("Initialized address merkle tree");
    merkle_tree.init().map_err(ProgramError::from)?;
    // Initialize the address merkle tree with the bn254 Fr field size - 1
    // This is the highest value that you can poseidon hash with poseidon syscalls.
    // Initializing the indexed Merkle tree enables non-inclusion proofs without handling the first case specifically.
    // However, it does reduce the available address space by 1.
    merkle_tree
        .add_highest_element()
        .map_err(ProgramError::from)?;

    Ok(())
}

use crate::{errors::AccountCompressionErrorCode, state::AddressMerkleTreeAccount};
pub use anchor_lang::prelude::*;
use light_indexed_merkle_tree::FIELD_SIZE_SUB_ONE;
use light_utils::fee::compute_rollover_fee;
use num_bigint::BigUint;
use num_traits::Num;

#[derive(Accounts)]
pub struct InitializeAddressMerkleTree<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(zero)]
    pub merkle_tree: AccountLoader<'info, AddressMerkleTreeAccount>,
}

pub fn process_initialize_address_merkle_tree(
    address_merkle_tree_loader: &AccountLoader<'_, AddressMerkleTreeAccount>,
    index: u64,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    height: u32,
    changelog_size: u64,
    roots_size: u64,
    canopy_depth: u64,
    address_changelog_size: u64,
    associated_queue: Pubkey,
    tip: u64,
    rollover_threshold: Option<u64>,
    close_threshold: Option<u64>,
    rent: u64,
) -> Result<()> {
    let mut address_merkle_tree = address_merkle_tree_loader.load_init()?;

    address_merkle_tree.index = index;
    address_merkle_tree.owner = owner;
    address_merkle_tree.delegate = delegate.unwrap_or_default();
    address_merkle_tree.tip = tip;
    address_merkle_tree.associated_queue = associated_queue;

    address_merkle_tree.rollover_fee = match rollover_threshold {
        Some(rollover_threshold) => {
            compute_rollover_fee(rollover_threshold, height, rent).map_err(ProgramError::from)?
        }
        None => 0,
    };
    address_merkle_tree.rollover_threshold = rollover_threshold.unwrap_or(u64::MAX);
    address_merkle_tree.rolledover_slot = u64::MAX;
    address_merkle_tree.close_threshold = close_threshold.unwrap_or(u64::MAX);

    address_merkle_tree
        .load_merkle_tree_init(
            height
                .try_into()
                .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?,
            changelog_size
                .try_into()
                .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?,
            roots_size
                .try_into()
                .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?,
            canopy_depth
                .try_into()
                .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?,
            address_changelog_size
                .try_into()
                .map_err(|_| AccountCompressionErrorCode::IntegerOverflow)?,
        )
        .map_err(ProgramError::from)?;
    let address_merkle_tree_inited = address_merkle_tree.load_merkle_tree_mut()?;

    // Initialize the address merkle tree with the bn254 Fr field size - 1
    // This is the highest value that you can poseidon hash with poseidon syscalls.
    // Initializing the indexed Merkle tree enables non-inclusion proofs without handling the first case specifically.
    // However, it does reduce the available address space by 1.
    let init_value = BigUint::from_str_radix(FIELD_SIZE_SUB_ONE, 10).unwrap();
    address_merkle_tree_inited
        .merkle_tree
        .initialize_address_merkle_tree(init_value)
        .map_err(ProgramError::from)?;
    Ok(())
}

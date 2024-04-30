use crate::{errors::AccountCompressionErrorCode, state::AddressMerkleTreeAccount};
pub use anchor_lang::prelude::*;
use light_indexed_merkle_tree::FIELD_SIZE_SUB_ONE;
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
    let total_number_of_leaves = 2u64.pow(height);

    address_merkle_tree.rollover_fee = match rollover_threshold {
        Some(rollover_threshold) => {
            compute_rollover_fee(rollover_threshold, total_number_of_leaves, rent)?
        }
        None => 0,
    };
    address_merkle_tree.rollover_threshold = rollover_threshold.unwrap_or(u64::MAX);
    address_merkle_tree.rolledover_slot = u64::MAX;
    address_merkle_tree.close_threshold = close_threshold.unwrap_or(u64::MAX);

    msg!("bazinga");
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

pub fn compute_rollover_fee(
    rollover_threshold: u64,
    number_of_transactions: u64,
    rent: u64,
) -> Result<u64> {
    if rollover_threshold > 100 {
        return err!(AccountCompressionErrorCode::RolloverThresholdTooHigh);
    }
    // rent / (total_number_of_leaves * (rollover_threshold / 100))
    // + 1 to pick the next fee that is higher than the rent
    Ok((rent * 100 / (number_of_transactions * rollover_threshold)) + 1)
}

#[test]
fn test_compute_rollover_fee() {
    let rollover_threshold = 100;
    let tree_height = 26;
    let rent = 1392890880;
    let total_number_of_leaves = 2u64.pow(tree_height);

    let fee = compute_rollover_fee(rollover_threshold, total_number_of_leaves, rent).unwrap();
    // assert_ne!(fee, 0u64);
    assert!((fee + 1) * (total_number_of_leaves * 100 / rollover_threshold) > rent);

    let rollover_threshold = 50;
    let fee = compute_rollover_fee(rollover_threshold, total_number_of_leaves, rent).unwrap();
    assert!((fee + 1) * (total_number_of_leaves * 100 / rollover_threshold) > rent);
    let rollover_threshold: u64 = 95;

    let fee = compute_rollover_fee(rollover_threshold, total_number_of_leaves, rent).unwrap();
    assert!((fee + 1) * (total_number_of_leaves * 100 / rollover_threshold) > rent);
}

/// Prints fees:
/// every input compressed account incurs a nullifier queue rollover fee
/// every output compressed account incurs a merkle tree rollover fee
#[test]
fn print_compute_rollover_fee() {
    use crate::{NullifierQueueConfig, StateMerkleTreeConfig};
    let tree_height = 26;
    let merkle_tree_rent: u64 = 9496725120;
    let total_number_of_leaves = 2u64.pow(tree_height);
    let merkle_tree_config = StateMerkleTreeConfig::default();

    let fee = compute_rollover_fee(
        merkle_tree_config.rollover_threshold.unwrap(),
        total_number_of_leaves,
        merkle_tree_rent,
    )
    .unwrap();
    println!("merkle tree config {:?}", merkle_tree_config);
    println!("merkle tree rollover fee: {}", fee);
    println!(
        "merkle tree rollover fee + tip: {}",
        fee + merkle_tree_config.tip.unwrap_or_default()
    );

    let nullifier_config = NullifierQueueConfig::default();
    let rent = 1796849280;

    let fee = compute_rollover_fee(
        merkle_tree_config.rollover_threshold.unwrap(),
        total_number_of_leaves,
        rent,
    )
    .unwrap();
    println!("nullifier queue config {:?}", nullifier_config);
    println!("nullifier queue rollover fee: {}", fee);
    println!(
        "nullifier queue rollover fee + tip: {}",
        fee + nullifier_config.tip.unwrap_or_default()
    );
    let rent_address_merkle_tree = 9496725120;
    let rent_address_queue = 1796849280;
    let rent = rent_address_merkle_tree + rent_address_queue;
    let fee = compute_rollover_fee(
        merkle_tree_config.rollover_threshold.unwrap(),
        total_number_of_leaves,
        rent,
    )
    .unwrap();
    println!("address merkle tree config {:?}", merkle_tree_config);
    println!("address queue config {:?}", nullifier_config);
    println!("address queue rollover fee: {}", fee);
    println!(
        "address queue rollover fee + tip: {}",
        fee + nullifier_config.tip.unwrap_or_default()
    );
}

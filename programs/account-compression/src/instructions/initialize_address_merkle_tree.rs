use crate::{errors::AccountCompressionErrorCode, state::AddressMerkleTreeAccount};
pub use anchor_lang::prelude::*;
use light_bounded_vec::BoundedVec;
use light_hasher::{zero_indexed_leaf::poseidon::ZERO_INDEXED_LEAF, Hasher, Poseidon};
use light_indexed_merkle_tree::array::IndexedArray;
use num_bigint::{BigUint, ToBigUint};
use num_traits::Num;
use std::ops::Sub;
#[derive(Accounts)]
pub struct InitializeAddressMerkleTree<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(zero)]
    pub merkle_tree: AccountLoader<'info, AddressMerkleTreeAccount>,
}

pub fn process_initialize_address_merkle_tree<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeAddressMerkleTree<'info>>,
    index: u64,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    height: u64,
    changelog_size: u64,
    roots_size: u64,
    canopy_depth: u64,
) -> Result<()> {
    let mut address_merkle_tree = ctx.accounts.merkle_tree.load_init()?;

    address_merkle_tree.index = index;
    address_merkle_tree.owner = owner;
    address_merkle_tree.delegate = delegate.unwrap_or(owner);

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
        )
        .map_err(ProgramError::from)?;
    let init_value = BigUint::from_str_radix(
        "21888242871839275222246405745257275088548364400416034343698204186575808495617",
        10,
    )
    .unwrap()
    .sub(1u32.to_biguint().unwrap());
    let mut indexed_array = IndexedArray::<light_hasher::Poseidon, usize, 2>::default();

    let nullifier_bundle = indexed_array.append(&init_value).unwrap();
    let address_merkle_tree_inited = address_merkle_tree.load_merkle_tree_mut().map_err(ProgramError::from)?;

    let new_low_leaf = nullifier_bundle
        .new_low_element
        .hash::<Poseidon>(&nullifier_bundle.new_element.value)
        .unwrap();
    let mut zero_bytes_array = BoundedVec::with_capacity(26);
    for i in 0..16 {
        zero_bytes_array.push(Poseidon::zero_bytes()[i]).unwrap();
    }

    address_merkle_tree_inited
        .merkle_tree
        .update(
            address_merkle_tree_inited.changelog_index(),
            &ZERO_INDEXED_LEAF,
            &new_low_leaf,
            0,
            &mut zero_bytes_array,
        )
        .unwrap();

    // Append new element.
    let new_leaf = nullifier_bundle
        .new_element
        .hash::<Poseidon>(&nullifier_bundle.new_element_next_value)
        .unwrap();
    address_merkle_tree_inited
        .merkle_tree
        .append(&new_leaf)
        .unwrap();
    Ok(())
}

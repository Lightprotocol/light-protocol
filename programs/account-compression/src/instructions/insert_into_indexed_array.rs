use std::collections::HashMap;

use aligned_sized::aligned_sized;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use ark_ff::BigInteger256;
use ark_serialize::CanonicalDeserialize;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::array::IndexingArray;

use crate::RegisteredProgram;

#[derive(Accounts)]
pub struct InsertIntoIndexedArrays<'info> {
    /// CHECK: should only be accessed by a registered program/owner/delegate.
    #[account(mut)]
    pub authority: Signer<'info>,
    pub registered_verifier_pda: Option<Account<'info, RegisteredProgram>>, // nullifiers are sent in remaining accounts. @ErrorCode::InvalidVerifier
}

/// Inserts every element into the indexed array.
/// Throws an error if the element already exists.
/// Expects an indexed queue account as for every index as remaining account.
pub fn process_insert_into_indexed_arrays<'a, 'info>(
    ctx: Context<'a, '_, '_, 'info, InsertIntoIndexedArrays<'info>>,
    elements: &'a [[u8; 32]],
    low_element_indexes: &'a [u16],
) -> Result<()> {
    if low_element_indexes.len() != elements.len() {
        msg!(
            "Number of low  does not match number elements {} != {}",
            low_element_indexes.len(),
            elements.len()
        );
        return err!(crate::errors::ErrorCode::NumberOfLeavesMismatch);
    }
    if elements.len() != ctx.remaining_accounts.len() {
        msg!(
            "Number of elements does not match number of indexed arrays accounts {} != {}",
            elements.len(),
            ctx.remaining_accounts.len()
        );
        return err!(crate::errors::ErrorCode::NumberOfLeavesMismatch);
    }
    // for every index
    let mut array_map = HashMap::<Pubkey, (&AccountInfo, Vec<[u8; 32]>, Vec<u16>)>::new();
    for (i, mt) in ctx.remaining_accounts.iter().enumerate() {
        match array_map.get(&mt.key()) {
            Some(_) => {}
            None => {
                array_map.insert(mt.key(), (mt, Vec::new(), Vec::new()));
            }
        };
        array_map.get_mut(&mt.key()).unwrap().1.push(elements[i]);
        array_map
            .get_mut(&mt.key())
            .unwrap()
            .2
            .push(low_element_indexes[i]);
    }

    for (mt, elements, low_element_indexes) in array_map.values() {
        let array = AccountLoader::<IndexedArrayAccount>::try_from(mt).unwrap();
        let mut array_account = array.load_mut()?;
        let array = indexed_array_from_bytes_mut(&mut array_account.indexed_array);
        for (element, _index) in elements.iter().zip(low_element_indexes) {
            msg!("Inserting element {:?} into indexed array", element);

            array
                .append(
                    // *index, TODO: enable index once we have rpc to get the low elements from indexer and a correction function inside the append function the index should just be a starting point
                    BigInteger256::deserialize_uncompressed_unchecked(element.as_slice()).unwrap(),
                )
                .unwrap();
            msg!(
                "Inserted elements into indexed arrays {:?}",
                array.elements[0]
            );
            msg!(
                "Inserted elements into indexed arrays 1 {:?}",
                array.elements[1]
            );
            msg!(
                "Inserted elements into indexed arrays  2 {:?}",
                array.elements[2]
            );
        }
    }
    msg!("Inserted elements into indexed arrays {:?}", elements);
    Ok(())
}

// TODO: add a function to merkle tree program that creates a new Merkle tree and indexed array account in the same transaction with consistent parameters and add them to the group
// we can use the same group regulate permissions for the de compression pool program
pub fn process_initialize_indexed_array<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeIndexedArrays<'info>>,
    index: u64,
    owner: Pubkey,
    delegate: Option<Pubkey>,
) -> Result<()> {
    let mut indexed_array_account = ctx.accounts.indexed_array.load_init()?;
    indexed_array_account.index = index;
    indexed_array_account.owner = owner;
    indexed_array_account.delegate = delegate.unwrap_or(owner);
    // Explicitly initializing the indexed array is not necessary as defautl values are all zero.
    Ok(())
}

#[derive(Accounts)]
pub struct InitializeIndexedArrays<'info> {
    pub authority: Signer<'info>,
    #[account(zero)]
    pub indexed_array: AccountLoader<'info, IndexedArrayAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Debug, PartialEq)]
#[account(zero_copy)]
#[aligned_sized(anchor)]
pub struct IndexedArrayAccount {
    pub index: u64,
    pub owner: Pubkey,
    pub delegate: Pubkey,
    pub array: Pubkey,
    pub indexed_array: [u8; 112008],
}

pub type IndexedArray = IndexingArray<Poseidon, BigInteger256, 2800>;

pub fn indexed_array_from_bytes(bytes: &[u8; 112008]) -> &IndexedArray {
    // SAFETY: We make sure that the size of the byte slice is equal to
    // the size of `IndexedArray`.
    // The only reason why we are doing this is that Anchor is struggling with
    // generating IDL when `ConcurrentMerkleTree` with generics is used
    // directly as a field.
    unsafe {
        let ptr = bytes.as_ptr() as *const IndexedArray;
        &*ptr
    }
}

pub fn indexed_array_from_bytes_mut(bytes: &mut [u8; 112008]) -> &mut IndexedArray {
    // SAFETY: We make sure that the size of the byte slice is equal to
    // the size of `IndexedArray`.
    // The only reason why we are doing this is that Anchor is struggling with
    // generating IDL when `ConcurrentMerkleTree` with generics is used
    // directly as a field.
    unsafe {
        let ptr = bytes.as_ptr() as *mut IndexedArray;
        &mut *ptr
    }
}

pub fn initialize_default_indexed_array(indexed_array: &mut [u8; 112008]) {
    // SAFETY: We make sure that the size of the byte slice is equal to
    // the size of `IndexedArray`.

    unsafe {
        let ptr = indexed_array.as_ptr() as *mut IndexedArray;
        // Assuming IndexedArray implements Default and Poseidon, BigInteger256 are types that fit into the generic parameters
        std::ptr::write(
            ptr,
            IndexingArray::<Poseidon, BigInteger256, 2800>::default(),
        );
    }
}

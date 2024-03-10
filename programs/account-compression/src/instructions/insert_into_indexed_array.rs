use std::collections::HashMap;

use aligned_sized::aligned_sized;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use ark_ff::BigInteger256;
use ark_serialize::CanonicalDeserialize;
use light_hasher::Poseidon;
use light_indexed_merkle_tree::array::IndexingArray;

use crate::{utils::constants::STATE_INDEXED_ARRAY_SIZE, RegisteredProgram};

#[derive(Accounts)]
pub struct InsertIntoIndexedArrays<'info> {
    /// CHECK: should only be accessed by a registered program/owner/delegate.
    #[account(mut)]
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>, // nullifiers are sent in remaining accounts. @ErrorCode::InvalidVerifier
}

/// Inserts every element into the indexed array.
/// Throws an error if the element already exists.
/// Expects an indexed queue account as for every index as remaining account.
pub fn process_insert_into_indexed_arrays<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, InsertIntoIndexedArrays<'info>>,
    elements: &'a [[u8; 32]],
) -> Result<()> {
    if elements.len() != ctx.remaining_accounts.len() {
        msg!(
            "Number of elements does not match number of indexed arrays accounts {} != {}",
            elements.len(),
            ctx.remaining_accounts.len()
        );
        return err!(crate::errors::AccountCompressionErrorCode::NumberOfLeavesMismatch);
    }
    // for every index
    let mut array_map = HashMap::<Pubkey, (&'info AccountInfo, Vec<[u8; 32]>)>::new();
    for (i, mt) in ctx.remaining_accounts.iter().enumerate() {
        match array_map.get(&mt.key()) {
            Some(_) => {}
            None => {
                array_map.insert(mt.key(), (mt, Vec::new()));
            }
        };
        array_map.get_mut(&mt.key()).unwrap().1.push(elements[i]);
    }

    for (mt, elements) in array_map.values() {
        msg!("Inserting into indexed array {:?}", mt.key());
        let array = AccountLoader::<IndexedArrayAccount>::try_from(mt).unwrap();
        let mut array_account = array.load_mut()?;
        let array = indexed_array_from_bytes_mut(&mut array_account.indexed_array);
        for element in elements.iter() {
            array
                .append(
                    BigInteger256::deserialize_uncompressed_unchecked(element.as_slice()).unwrap(),
                )
                .unwrap();
        }
    }
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
    pub indexed_array: [u8; 192008],
}

pub type IndexedArray = IndexingArray<Poseidon, u16, BigInteger256, STATE_INDEXED_ARRAY_SIZE>;

pub fn indexed_array_from_bytes(bytes: &[u8; 192008]) -> &IndexedArray {
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

pub fn indexed_array_from_bytes_mut(bytes: &mut [u8; 192008]) -> &mut IndexedArray {
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

pub fn initialize_default_indexed_array(indexed_array: &mut [u8; 192008]) {
    // SAFETY: We make sure that the size of the byte slice is equal to
    // the size of `IndexedArray`.

    unsafe {
        let ptr = indexed_array.as_ptr() as *mut IndexedArray;
        // Assuming IndexedArray implements Default and Poseidon, BigInteger256 are types that fit into the generic parameters
        std::ptr::write(
            ptr,
            IndexingArray::<Poseidon, u16, BigInteger256, STATE_INDEXED_ARRAY_SIZE>::default(),
        );
    }
}

#[cfg(not(target_os = "solana"))]
pub mod indexed_array_sdk {
    use anchor_lang::{system_program, InstructionData};
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    };

    pub fn create_initialize_indexed_array_instruction(
        payer: Pubkey,
        indexed_array_pubkey: Pubkey,
        index: u64,
    ) -> Instruction {
        let instruction_data: crate::instruction::InitializeIndexedArray =
            crate::instruction::InitializeIndexedArray {
                index,
                owner: payer,
                delegate: None,
            };
        Instruction {
            program_id: crate::ID,
            accounts: vec![
                AccountMeta::new(payer, true),
                AccountMeta::new(indexed_array_pubkey, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: instruction_data.data(),
        }
    }
}

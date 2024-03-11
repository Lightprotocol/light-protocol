use std::collections::HashMap;

use aligned_sized::aligned_sized;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use bytemuck::{Pod, Zeroable};

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
        for element in elements.iter() {
            msg!("Inserting element {:?}", element);
            let insert_index = array_account.non_inclusion(element, &0usize)?;
            array_account.indexed_array[insert_index].element = *element;
            array_account.indexed_array[insert_index].merkle_tree_overwrite_sequence_number = 0u64;
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
    pub indexed_array: [QueueArrayElemenet; STATE_INDEXED_ARRAY_SIZE],
}

#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy, AnchorSerialize, AnchorDeserialize, Zeroable, Pod)]
pub struct QueueArrayElemenet {
    /// The squence number of the Merkle tree at which it is safe to overwrite the element.
    /// It is safe to overwrite an element once no root that includes the element is in the root history array.
    /// With every time a root is inserted into the root history array, the sequence number is incremented.
    /// 0 means that the element still exists in the state Merkle tree, is not nullified yet.
    /// TODO: add a root history array sequence number to the Merkle tree account.
    pub merkle_tree_overwrite_sequence_number: u64,
    pub element: [u8; 32],
}

impl IndexedArrayAccount {
    /// Naive non-inclusion check remove once hash set is ready.
    pub fn non_inclusion(
        &self,
        value: &[u8; 32],
        current_sequence_number: &usize,
    ) -> Result<usize> {
        for (i, element) in self.indexed_array.iter().enumerate() {
            if element.element == *value {
                return Err(
                    crate::errors::AccountCompressionErrorCode::ElementAlreadyExists.into(),
                );
            }
            // TODO: make sure that there is no vulnerability for a fresh array and tree.
            else if element.merkle_tree_overwrite_sequence_number
                < *current_sequence_number as u64
                || element.element == [0; 32]
            {
                return Ok(i);
            }
        }
        Err(crate::errors::AccountCompressionErrorCode::HashSetFull.into())
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

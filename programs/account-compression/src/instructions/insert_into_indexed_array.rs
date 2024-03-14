use std::{cell::RefMut, mem};

use aligned_sized::aligned_sized;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_hash_set::{HashSet, HashSetZeroCopy};
use num_bigint::BigUint;

use crate::{
    errors::AccountCompressionErrorCode,
    utils::{
        check_registered_or_signer::{check_registered_or_signer, GroupAccess, GroupAccounts},
        queue::{QueueBundle, QueueMap},
    },
    RegisteredProgram, StateMerkleTreeAccount,
};

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
    let expected_remaining_accounts = elements.len() * 2;
    if expected_remaining_accounts != ctx.remaining_accounts.len() {
        msg!(
            "Number of remaining accounts does not match, expected {}, got {}",
            expected_remaining_accounts,
            ctx.remaining_accounts.len()
        );
        return err!(crate::errors::AccountCompressionErrorCode::NumberOfLeavesMismatch);
    }

    let mut queue_map = QueueMap::new();
    for i in 0..elements.len() {
        let queue = ctx.remaining_accounts.get(i).unwrap();
        let merkle_tree = ctx.remaining_accounts.get(elements.len() + i).unwrap();

        queue_map
            .entry(queue.key())
            .or_insert_with(|| QueueBundle::new(queue, merkle_tree))
            .elements
            .push(elements[i]);
    }

    for queue_bundle in queue_map.values() {
        msg!(
            "Inserting into indexed array {:?}",
            queue_bundle.queue.key()
        );

        let indexed_array = AccountLoader::<IndexedArrayAccount>::try_from(queue_bundle.queue)?;
        {
            let indexed_array = indexed_array.load()?;
            check_registered_or_signer::<InsertIntoIndexedArrays, IndexedArrayAccount>(
                &ctx,
                &indexed_array,
            )?;
            if queue_bundle.merkle_tree.key() != indexed_array.associated_merkle_tree {
                return err!(AccountCompressionErrorCode::InvalidMerkleTree);
            }
        }

        let merkle_tree =
            AccountLoader::<StateMerkleTreeAccount>::try_from(queue_bundle.merkle_tree)?;
        let sequence_number = {
            let merkle_tree = merkle_tree.load()?;
            merkle_tree.load_merkle_tree()?.sequence_number
        };

        let indexed_array = indexed_array.to_account_info();
        let mut indexed_array = indexed_array.try_borrow_mut_data()?;
        let mut indexed_array =
            unsafe { indexed_array_from_bytes_zero_copy_mut(&mut indexed_array).unwrap() };

        for element in queue_bundle.elements.iter() {
            msg!("Inserting element {:?}", element);
            let element = BigUint::from_bytes_be(element.as_slice());
            indexed_array
                .insert(&element, sequence_number)
                .map_err(ProgramError::from)?;
        }
    }

    Ok(())
}

// TODO: add a function to merkle tree program that creates a new Merkle tree and indexed array account in the same transaction with consistent parameters and add them to the group
// we can use the same group regulate permissions for the de compression pool program
pub fn process_initialize_indexed_array<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, InitializeIndexedArrays<'info>>,
    index: u64,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    associated_merkle_tree: Option<Pubkey>,
    capacity_indices: u16,
    capacity_values: u16,
    sequence_threshold: u64,
) -> Result<()> {
    {
        let mut indexed_array_account = ctx.accounts.indexed_array.load_init()?;
        indexed_array_account.index = index;
        indexed_array_account.owner = owner;
        indexed_array_account.delegate = delegate.unwrap_or(owner);
        indexed_array_account.associated_merkle_tree = associated_merkle_tree.unwrap_or_default();
        drop(indexed_array_account);
    }

    let indexed_array = ctx.accounts.indexed_array.to_account_info();
    let mut indexed_array = indexed_array.try_borrow_mut_data()?;
    let _ = unsafe {
        indexed_array_from_bytes_zero_copy_init(
            &mut indexed_array,
            capacity_indices.into(),
            capacity_values.into(),
            sequence_threshold as usize,
        )
        .unwrap()
    };

    // Explicitly initializing the indexed array is not necessary as default values are all zero.
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
    pub associated_merkle_tree: Pubkey,
}

impl GroupAccess for IndexedArrayAccount {
    fn get_owner(&self) -> &Pubkey {
        &self.owner
    }

    fn get_delegate(&self) -> &Pubkey {
        &self.delegate
    }
}

impl<'info> GroupAccounts<'info> for InsertIntoIndexedArrays<'info> {
    fn get_signing_address(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

impl IndexedArrayAccount {
    pub fn size(capacity_indices: usize, capacity_values: usize) -> Result<usize> {
        Ok(8 + mem::size_of::<Self>()
            + HashSet::<u16>::size_in_account(capacity_indices, capacity_values)
                .map_err(ProgramError::from)?)
    }
}

/// Creates a copy of `IndexedArray` from the given account data.
///
/// # Safety
///
/// This operation is unsafe. It's the caller's responsibility to ensure that
/// the provided account data have correct size and alignment.
pub unsafe fn indexed_array_from_bytes_copy(
    mut data: RefMut<'_, &mut [u8]>,
    // data: &'a mut [u8],
) -> Result<HashSet<u16>> {
    let data = &mut data[8 + mem::size_of::<IndexedArrayAccount>()..];
    let queue = HashSet::<u16>::from_bytes_copy(data).map_err(ProgramError::from)?;
    Ok(queue)
}

/// Casts the given account data to an `IndexedArrayZeroCopy` instance.
///
/// # Safety
///
/// This operation is unsafe. It's the caller's responsibility to ensure that
/// the provided account data have correct size and alignment.
pub unsafe fn indexed_array_from_bytes_zero_copy_mut(
    data: &mut [u8],
) -> Result<HashSetZeroCopy<u16>> {
    let data = &mut data[8 + mem::size_of::<IndexedArrayAccount>()..];
    let queue =
        HashSetZeroCopy::<u16>::from_bytes_zero_copy_mut(data).map_err(ProgramError::from)?;
    Ok(queue)
}

/// Casts the given account data to an `IndexedArrayZeroCopy` instance.
///
/// # Safety
///
/// This operation is unsafe. It's the caller's responsibility to ensure that
/// the provided account data have correct size and alignment.
pub unsafe fn indexed_array_from_bytes_zero_copy_init(
    data: &mut [u8],
    capacity_indices: usize,
    capacity_values: usize,
    sequence_threshold: usize,
) -> Result<HashSetZeroCopy<u16>> {
    let data = &mut data[8 + mem::size_of::<IndexedArrayAccount>()..];
    let queue = HashSetZeroCopy::<u16>::from_bytes_zero_copy_init(
        data,
        capacity_indices,
        capacity_values,
        sequence_threshold,
    )
    .map_err(ProgramError::from)?;
    Ok(queue)
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
        associated_merkle_tree: Option<Pubkey>,
        capacity_indices: u16,
        capacity_values: u16,
        sequence_threshold: u64,
    ) -> Instruction {
        let instruction_data: crate::instruction::InitializeIndexedArray =
            crate::instruction::InitializeIndexedArray {
                index,
                owner: payer,
                delegate: None,
                associated_merkle_tree,
                capacity_indices,
                capacity_values,
                sequence_threshold,
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

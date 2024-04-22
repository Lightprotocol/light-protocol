use std::{cell::RefMut, mem};

use aligned_sized::aligned_sized;
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_hash_set::{zero_copy::HashSetZeroCopy, HashSet};
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
pub struct InsertIntoNullifierQueues<'info> {
    /// CHECK: should only be accessed by a registered program/owner/delegate.
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>, // nullifiers are sent in remaining accounts. @ErrorCode::InvalidVerifier
}

/// Inserts every element into the nullifier queue.
/// Throws an error if the element already exists.
/// Expects a nullifier queue account as for every index as remaining account.
pub fn process_insert_into_nullifier_queues<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, InsertIntoNullifierQueues<'info>>,
    elements: &'a [[u8; 32]],
) -> Result<()> {
    let expected_remaining_accounts = elements.len() * 2;
    if expected_remaining_accounts != ctx.remaining_accounts.len() {
        msg!(
            "Number of remaining accounts does not match, expected {}, got {}",
            expected_remaining_accounts,
            ctx.remaining_accounts.len()
        );
        return err!(AccountCompressionErrorCode::NumberOfLeavesMismatch);
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
            "Inserting into nullifier queue {:?}",
            queue_bundle.queue.key()
        );

        let nullifier_queue = AccountLoader::<NullifierQueueAccount>::try_from(queue_bundle.queue)?;
        {
            let nullifier_queue = nullifier_queue.load()?;
            check_registered_or_signer::<InsertIntoNullifierQueues, NullifierQueueAccount>(
                &ctx,
                &nullifier_queue,
            )?;
            if queue_bundle.merkle_tree.key() != nullifier_queue.associated_merkle_tree {
                return err!(AccountCompressionErrorCode::InvalidMerkleTree);
            }
        }

        let merkle_tree =
            AccountLoader::<StateMerkleTreeAccount>::try_from(queue_bundle.merkle_tree)?;
        let sequence_number = {
            let merkle_tree = merkle_tree.load()?;
            merkle_tree.load_merkle_tree()?.sequence_number
        };

        let nullifier_queue = nullifier_queue.to_account_info();
        let mut nullifier_queue = nullifier_queue.try_borrow_mut_data()?;
        let mut nullifier_queue =
            unsafe { nullifier_queue_from_bytes_zero_copy_mut(&mut nullifier_queue).unwrap() };

        for element in queue_bundle.elements.iter() {
            msg!("Inserting element {:?}", element);
            let element = BigUint::from_bytes_be(element.as_slice());
            nullifier_queue
                .insert(&element, sequence_number)
                .map_err(ProgramError::from)?;
        }
    }

    Ok(())
}

// TODO: add a function to merkle tree program that creates a new Merkle tree and nullifier queue account \
// in the same transaction with consistent parameters and add them to the group
// we can use the same group regulate permissions for the de compression pool program
pub fn process_initialize_nullifier_queue<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, InitializeNullifierQueues<'info>>,
    index: u64,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    associated_merkle_tree: Option<Pubkey>,
    capacity_indices: u16,
    capacity_values: u16,
    sequence_threshold: u64,
) -> Result<()> {
    {
        let mut nullifier_queue_account = ctx.accounts.nullifier_queue.load_init()?;
        nullifier_queue_account.index = index;
        nullifier_queue_account.owner = owner;
        nullifier_queue_account.delegate = delegate.unwrap_or(owner);
        nullifier_queue_account.associated_merkle_tree = associated_merkle_tree.unwrap_or_default();
        drop(nullifier_queue_account);
    }

    let nullifier_queue = ctx.accounts.nullifier_queue.to_account_info();
    let mut nullifier_queue = nullifier_queue.try_borrow_mut_data()?;
    let _ = unsafe {
        nullifier_queue_from_bytes_zero_copy_init(
            &mut nullifier_queue,
            capacity_indices.into(),
            capacity_values.into(),
            sequence_threshold as usize,
        )
        .unwrap()
    };

    // Explicitly initializing the nullifier queue is not necessary as default values are all zero.
    Ok(())
}

#[derive(Accounts)]
pub struct InitializeNullifierQueues<'info> {
    pub authority: Signer<'info>,
    #[account(zero)]
    pub nullifier_queue: AccountLoader<'info, NullifierQueueAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Debug, PartialEq)]
#[account(zero_copy)]
#[aligned_sized(anchor)]
pub struct NullifierQueueAccount {
    pub index: u64,
    pub owner: Pubkey,
    pub delegate: Pubkey,
    pub associated_merkle_tree: Pubkey,
}

impl GroupAccess for NullifierQueueAccount {
    fn get_owner(&self) -> &Pubkey {
        &self.owner
    }

    fn get_delegate(&self) -> &Pubkey {
        &self.delegate
    }
}

impl<'info> GroupAccounts<'info> for InsertIntoNullifierQueues<'info> {
    fn get_signing_address(&self) -> &Signer<'info> {
        &self.authority
    }
    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

impl NullifierQueueAccount {
    pub fn size(capacity_indices: usize, capacity_values: usize) -> Result<usize> {
        Ok(8 + mem::size_of::<Self>()
            + HashSet::<u16>::size_in_account(capacity_indices, capacity_values)
                .map_err(ProgramError::from)?)
    }
}

/// Creates a copy of `NullifierQueue` from the given account data.
///
/// # Safety
///
/// This operation is unsafe. It's the caller's responsibility to ensure that
/// the provided account data have correct size and alignment.
pub unsafe fn nullifier_queue_from_bytes_copy(
    mut data: RefMut<'_, &mut [u8]>,
    // data: &'a mut [u8],
) -> Result<HashSet<u16>> {
    let data = &mut data[8 + mem::size_of::<NullifierQueueAccount>()..];
    let queue = HashSet::<u16>::from_bytes_copy(data).map_err(ProgramError::from)?;
    Ok(queue)
}

/// Casts the given account data to an `HashSetZeroCopy` instance.
///
/// # Safety
///
/// This operation is unsafe. It's the caller's responsibility to ensure that
/// the provided account data have correct size and alignment.
pub unsafe fn nullifier_queue_from_bytes_zero_copy_mut(
    data: &mut [u8],
) -> Result<HashSetZeroCopy<u16>> {
    let data = &mut data[8 + mem::size_of::<NullifierQueueAccount>()..];
    let queue =
        HashSetZeroCopy::<u16>::from_bytes_zero_copy_mut(data).map_err(ProgramError::from)?;
    Ok(queue)
}

/// Casts the given account data to an `HashSetZeroCopy` instance.
///
/// # Safety
///
/// This operation is unsafe. It's the caller's responsibility to ensure that
/// the provided account data have correct size and alignment.
pub unsafe fn nullifier_queue_from_bytes_zero_copy_init(
    data: &mut [u8],
    capacity_indices: usize,
    capacity_values: usize,
    sequence_threshold: usize,
) -> Result<HashSetZeroCopy<u16>> {
    let data = &mut data[8 + mem::size_of::<NullifierQueueAccount>()..];
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
pub mod nullifier_queue_sdk {
    use anchor_lang::{system_program, InstructionData};
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    };

    pub fn create_initialize_nullifier_queue_instruction(
        payer: Pubkey,
        nullifier_queue_pubkey: Pubkey,
        index: u64,
        associated_merkle_tree: Option<Pubkey>,
        capacity_indices: u16,
        capacity_values: u16,
        sequence_threshold: u64,
    ) -> Instruction {
        let instruction_data: crate::instruction::InitializeNullifierQueue =
            crate::instruction::InitializeNullifierQueue {
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
                AccountMeta::new(nullifier_queue_pubkey, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: instruction_data.data(),
        }
    }
}

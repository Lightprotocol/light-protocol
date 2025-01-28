use std::collections::HashMap;

use anchor_lang::{
    prelude::*,
    solana_program::{log::sol_log_compute_units, pubkey::Pubkey},
    Discriminator,
};
use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_hasher::Discriminator as LightDiscriminator;
use light_merkle_tree_metadata::queue::{check_queue_type, QueueType};
use num_bigint::BigUint;

use crate::{
    errors::AccountCompressionErrorCode,
    state::queue::{queue_from_bytes_zero_copy_mut, QueueAccount},
    state_merkle_tree_from_bytes_zero_copy,
    utils::{
        check_signer_is_registered_or_authority::check_signer_is_registered_or_authority,
        queue::{QueueBundle, QueueMap},
        transfer_lamports::transfer_lamports_cpi,
    },
    RegisteredProgram,
};

#[derive(Accounts)]
pub struct InsertIntoQueues<'info> {
    /// Fee payer pays rollover fee.
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK: should only be accessed by a registered program or owner.
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    pub system_program: Program<'info, System>,
}

/// Insert elements into the queues.
/// 1. Deduplicate tree and queue pairs.
///     1.1 select logic to create queue element
///         based on the account discriminator.
/// 2. Check that all leaves are processed.
/// 3. For each queue bundle:
///     3.1 Process bundle
///         3.1.1 Check account discriminators and account ownership.
///         3.1.2 Check accounts are associated.
///         3.1.3 Check that the signer is the authority or registered program.
///         3.1.4 Insert elements into the queue.
///     3.2 Transfer rollover fee.
pub fn process_insert_into_queues<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, InsertIntoQueues<'info>>,
    elements: &[[u8; 32]],
    indices: Vec<u32>,
    queue_type: QueueType,
    prove_by_index: Option<Vec<bool>>,
    tx_hash: Option<[u8; 32]>,
) -> Result<()> {
    if elements.is_empty() {
        return err!(AccountCompressionErrorCode::InputElementsEmpty);
    }

    light_heap::bench_sbf_start!("acp_create_queue_map");

    let mut queue_map = QueueMap::new();
    // 1. Deduplicate tree and queue pairs.
    //      So that we iterate over every pair only once,
    //      and pay rollover fees only once.
    let mut current_index = 0;
    for (index, element) in elements.iter().enumerate() {
        let current_account_discriminator = ctx
            .remaining_accounts
            .get(current_index)
            .unwrap()
            .try_borrow_data()?[0..8]
            .try_into()
            .unwrap();
        // 1.1 select logic to create queue element
        //       based on the account discriminator.
        match current_account_discriminator {
            // V1 nullifier or address queue.
            QueueAccount::DISCRIMINATOR => {
                if queue_type == QueueType::NullifierQueue
                    && prove_by_index.as_ref().unwrap()[index]
                {
                    return err!(AccountCompressionErrorCode::V1AccountMarkedAsProofByIndex);
                }

                add_queue_bundle_v1(
                    &mut current_index,
                    queue_type,
                    &mut queue_map,
                    element,
                    ctx.remaining_accounts,
                )?
            }
            // V2 nullifier (input state) queue
            BatchedQueueAccount::DISCRIMINATOR => add_nullifier_queue_bundle_v2(
                &mut current_index,
                queue_type,
                &mut queue_map,
                element,
                indices[index],
                prove_by_index.as_ref().unwrap()[index],
                ctx.remaining_accounts,
            )?,
            // V2 Address queue is part of the address Merkle tree account.
            BatchedMerkleTreeAccount::DISCRIMINATOR => add_address_queue_bundle_v2(
                &mut current_index,
                queue_type,
                &mut queue_map,
                element,
                ctx.remaining_accounts,
            )?,
            _ => {
                msg!(
                    "Invalid account discriminator {:?}",
                    current_account_discriminator
                );
                return err!(anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch);
            }
        }
    }

    // 2. Check that all leaves are processed.
    if current_index != ctx.remaining_accounts.len() {
        msg!(
            "Number of remaining accounts does not match, expected {}, got {}",
            current_index,
            ctx.remaining_accounts.len()
        );
        return err!(crate::errors::AccountCompressionErrorCode::NumberOfLeavesMismatch);
    }

    light_heap::bench_sbf_end!("acp_create_queue_map");

    for queue_bundle in queue_map.values() {
        // 3.1 Process bundle
        let rollover_fee = match queue_bundle.queue_type {
            QueueType::NullifierQueue => process_queue_bundle_v1(&ctx, queue_bundle),
            QueueType::AddressQueue => process_queue_bundle_v1(&ctx, queue_bundle),
            QueueType::BatchedInput => {
                process_nullifier_queue_bundle_v2(&ctx, queue_bundle, &tx_hash)
            }
            QueueType::BatchedAddress => process_address_queue_bundle_v2(&ctx, queue_bundle),
            _ => {
                msg!("Queue type {:?} is not supported", queue_bundle.queue_type);
                return err!(AccountCompressionErrorCode::InvalidQueueType);
            }
        }?;

        // 3.2 Transfer rollover fee.
        if rollover_fee > 0 {
            transfer_lamports_cpi(
                &ctx.accounts.fee_payer,
                &queue_bundle.accounts[0].to_account_info(),
                rollover_fee,
            )?;
        }
    }

    Ok(())
}

/// Process a v1 nullifier or address queue bundle.
/// 1. Check queue discriminator and account ownership
///     (AccountLoader).
/// 2. Check that queue has expected queue type.
/// 3. Check queue and Merkle tree are associated.
/// 4. Check that the signer is the authority or registered program.
/// 5. Insert the nullifiers into the queues hash set.
/// 6. Return rollover fee.
fn process_queue_bundle_v1<'info>(
    ctx: &Context<'_, '_, '_, 'info, InsertIntoQueues<'info>>,
    queue_bundle: &QueueBundle<'_, '_>,
) -> Result<u64> {
    // 1. Check discriminator and account ownership
    let queue = AccountLoader::<QueueAccount>::try_from(queue_bundle.accounts[0])?;
    let merkle_tree = queue_bundle.accounts[1];
    let associated_merkle_tree = {
        let queue = queue.load()?;
        // 2. Check that queue has expected queue type.
        check_queue_type(&queue.metadata.queue_type, &queue_bundle.queue_type)
            .map_err(ProgramError::from)?;
        queue.metadata.associated_merkle_tree
    };
    // 3. Check queue and Merkle tree are associated.
    if merkle_tree.key() != associated_merkle_tree.into() {
        msg!(
            "Queue account {:?} is not associated with Merkle tree  {:?}",
            queue.key(),
            merkle_tree.key()
        );
        return err!(AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated);
    }
    light_heap::bench_sbf_start!("acp_prep_insertion");
    let rollover_fee = {
        let queue = queue.load()?;
        // 4. Check that the signer is the authority or registered program.
        check_signer_is_registered_or_authority::<InsertIntoQueues, QueueAccount>(ctx, &queue)?;
        queue.metadata.rollover_metadata.rollover_fee * queue_bundle.elements.len() as u64
    };
    // 5. Insert the nullifiers into the queues hash set.
    {
        let sequence_number = {
            let merkle_tree = merkle_tree.try_borrow_data()?;
            let merkle_tree = state_merkle_tree_from_bytes_zero_copy(&merkle_tree)?;
            merkle_tree.sequence_number()
        };

        let queue = queue.to_account_info();
        let mut queue = queue.try_borrow_mut_data()?;
        let mut queue = unsafe { queue_from_bytes_zero_copy_mut(&mut queue).unwrap() };
        light_heap::bench_sbf_end!("acp_prep_insertion");
        light_heap::bench_sbf_start!("acp_insert_nf_into_queue");
        for element in queue_bundle.elements.iter() {
            let element = BigUint::from_bytes_be(element.as_slice());
            queue
                .insert(&element, sequence_number)
                .map_err(ProgramError::from)?;
        }
        light_heap::bench_sbf_end!("acp_insert_nf_into_queue");
    }
    // 6. Return rollover fee.
    Ok(rollover_fee)
}

/// Insert nullifiers into the batched nullifier queue.
/// 1. Check discriminator & account ownership
///     (state_from_account_info).
/// 2. Check discriminator & account ownership
///    (output_from_account_info).
/// 3. Check queue and Merkle tree are associated.
/// 3. Check that the signer is the authority or registered program.
/// 4. prove inclusion by index and zero out the leaf.
///     Note that this check doesn't fail if
///     the leaf index of the element is out of range for the queue.
///     This check needs to be relaxed since we want to use
///     compressed account which are in the Merkle tree
///     not only in the queue.
/// 5. Insert the nullifiers into the current input queue batch.
/// 6. Return rollover fee.
fn process_nullifier_queue_bundle_v2<'info>(
    ctx: &Context<'_, '_, '_, 'info, InsertIntoQueues<'info>>,
    queue_bundle: &QueueBundle<'_, '_>,
    tx_hash: &Option<[u8; 32]>,
) -> Result<u64> {
    // 1. Check discriminator & account ownership of Merkle tree.
    let merkle_tree =
        &mut BatchedMerkleTreeAccount::state_from_account_info(queue_bundle.accounts[0])
            .map_err(ProgramError::from)?;

    // 2. Check discriminator & account ownership of output queue.
    let output_queue = &mut BatchedQueueAccount::output_from_account_info(queue_bundle.accounts[1])
        .map_err(ProgramError::from)?;

    // 3. Check queue and Merkle tree are associated.
    output_queue
        .check_is_associated(&queue_bundle.accounts[0].key().into())
        .map_err(ProgramError::from)?;

    // 4. Check that the signer is the authority or registered program.
    check_signer_is_registered_or_authority::<InsertIntoQueues, BatchedMerkleTreeAccount>(
        ctx,
        merkle_tree,
    )?;

    for ((element, leaf_index), prove_by_index) in queue_bundle
        .elements
        .iter()
        .zip(queue_bundle.indices.iter())
        .zip(queue_bundle.prove_by_index.iter())
    {
        let tx_hash = tx_hash.ok_or(AccountCompressionErrorCode::TxHashUndefined)?;
        light_heap::bench_sbf_start!("acp_insert_nf_into_queue_v2");
        // 4. check for every account whether the value is still in the queue and zero it out.
        //      If checked fail if the value is not in the queue.
        output_queue
            .prove_inclusion_by_index_and_zero_out_leaf(
                *leaf_index as u64,
                element,
                *prove_by_index,
            )
            .map_err(ProgramError::from)?;

        // 5. Insert the nullifiers into the current input queue batch.
        merkle_tree
            .insert_nullifier_into_current_batch(element, *leaf_index as u64, &tx_hash)
            .map_err(ProgramError::from)?;
        light_heap::bench_sbf_end!("acp_insert_nf_into_queue_v2");
    }
    // 6. Return rollover fee.
    let rollover_fee =
        merkle_tree.metadata.rollover_metadata.rollover_fee * queue_bundle.elements.len() as u64;
    Ok(rollover_fee)
}

/// Insert a batch of addresses into the address queue.
/// 1. Check discriminator and account ownership.
/// 2. Check that the signer is the authority or registered program.
/// 3. Insert the addresses into the current batch.
/// 4. Return rollover fee.
fn process_address_queue_bundle_v2<'info>(
    ctx: &Context<'_, '_, '_, 'info, InsertIntoQueues<'info>>,
    queue_bundle: &QueueBundle<'_, '_>,
) -> Result<u64> {
    // 1. Check discriminator and account ownership.
    let merkle_tree =
        &mut BatchedMerkleTreeAccount::address_from_account_info(queue_bundle.accounts[0])
            .map_err(ProgramError::from)?;
    // 2. Check that the signer is the authority or registered program.
    check_signer_is_registered_or_authority::<InsertIntoQueues, BatchedMerkleTreeAccount>(
        ctx,
        merkle_tree,
    )?;
    // 3. Insert the addresses into the current batch.
    for element in queue_bundle.elements.iter() {
        light_heap::bench_sbf_start!("acp_insert_nf_into_queue_v2");
        merkle_tree
            .insert_address_into_current_batch(element)
            .map_err(ProgramError::from)?;
        light_heap::bench_sbf_end!("acp_insert_nf_into_queue_v2");
    }
    // 4. Return rollover fee.
    let rollover_fee =
        merkle_tree.metadata.rollover_metadata.rollover_fee * queue_bundle.elements.len() as u64;
    Ok(rollover_fee)
}

/// Add to/create new queue bundle for v1 nullifier or address queue.
fn add_queue_bundle_v1<'a, 'info>(
    remaining_accounts_index: &mut usize,
    queue_type: QueueType,
    queue_map: &mut HashMap<Pubkey, QueueBundle<'a, 'info>>,
    element: &'a [u8; 32],
    remaining_accounts: &'info [AccountInfo<'info>],
) -> Result<()> {
    let queue = remaining_accounts.get(*remaining_accounts_index).unwrap();
    let merkle_tree = remaining_accounts
        .get(*remaining_accounts_index + 1)
        .unwrap();
    queue_map
        .entry(queue.key())
        .or_insert_with(|| QueueBundle::new(queue_type, vec![queue, merkle_tree]))
        .elements
        .push(element);
    *remaining_accounts_index += 2;
    Ok(())
}

/// Add to/create a new state queue bundle.
/// 1. Check that the queue type is a nullifier queue.
/// 2. Get or create a queue bundle.
/// 3. Add the element to the queue bundle.
/// 4. Add the index to the queue bundle.
fn add_nullifier_queue_bundle_v2<'a, 'info>(
    remaining_accounts_index: &mut usize,
    queue_type: QueueType,
    queue_map: &mut HashMap<Pubkey, QueueBundle<'a, 'info>>,
    element: &'a [u8; 32],
    index: u32,
    prove_by_index: bool,
    remaining_accounts: &'info [AccountInfo<'info>],
) -> Result<()> {
    // 1. Check that the queue type is a nullifier queue.
    // Queue type is v1 nullifier queue type since we are using the same
    // instruction with both tree types via cpi from the system program.
    //  (sanity check)
    if queue_type != QueueType::NullifierQueue {
        return err!(AccountCompressionErrorCode::InvalidQueueType);
    }
    let output_queue = remaining_accounts.get(*remaining_accounts_index).unwrap();
    let merkle_tree = remaining_accounts
        .get(*remaining_accounts_index + 1)
        .unwrap();
    msg!("hashsetinsert");
    sol_log_compute_units();
    // 2. Get or create a queue bundle.
    // 3. Add the element to the queue bundle.
    queue_map
        .entry(merkle_tree.key())
        .or_insert_with(|| {
            QueueBundle::new(QueueType::BatchedInput, vec![merkle_tree, output_queue])
        })
        .elements
        .push(element);
    sol_log_compute_units();
    // 4. Add the index and proof by index to the queue bundle.
    queue_map.entry(merkle_tree.key()).and_modify(|x| {
        x.indices.push(index);
        x.prove_by_index.push(prove_by_index);
    });
    sol_log_compute_units();
    *remaining_accounts_index += 2;

    Ok(())
}

/// Add to/create a new address queue bundle.
/// 1. Check that the queue type is an address queue.
/// 2. Check that the Merkle tree is passed twice.
/// 3. Add the address to or create new queue bundle.
fn add_address_queue_bundle_v2<'a, 'info>(
    remaining_accounts_index: &mut usize,
    queue_type: QueueType,
    queue_map: &mut HashMap<Pubkey, QueueBundle<'a, 'info>>,
    address: &'a [u8; 32],
    remaining_accounts: &'info [AccountInfo<'info>],
) -> Result<()> {
    // 1. Check that the queue type is an address queue.
    //    (sanity check)
    if queue_type != QueueType::AddressQueue {
        return err!(AccountCompressionErrorCode::InvalidQueueType);
    }
    let merkle_tree = remaining_accounts.get(*remaining_accounts_index).unwrap();

    // 2. Check that the Merkle tree is passed twice.
    // We pass the same pubkey twice for consistency with the
    // nullification and address v1 instructions.
    if merkle_tree.key()
        != remaining_accounts
            .get(*remaining_accounts_index + 1)
            .unwrap()
            .key()
    {
        msg!(
            "Merkle tree accounts {:?} inconsistent {:?}",
            merkle_tree.key(),
            remaining_accounts
                .get(*remaining_accounts_index + 1)
                .unwrap()
                .key()
        );
        return err!(AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated);
    }
    // 3. Add the address to or create new queue bundle.
    queue_map
        .entry(merkle_tree.key())
        .or_insert_with(|| QueueBundle::new(QueueType::BatchedAddress, vec![merkle_tree]))
        .elements
        .push(address);
    *remaining_accounts_index += 2;

    Ok(())
}

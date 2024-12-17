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
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey, Discriminator, ZeroCopy};
use light_batched_merkle_tree::{
    merkle_tree::{BatchedMerkleTreeAccount, ZeroCopyBatchedMerkleTreeAccount},
    queue::{BatchedQueueAccount, ZeroCopyBatchedQueueAccount},
};
use light_hasher::Discriminator as LightDiscriminator;
use light_merkle_tree_metadata::queue::{check_queue_type, QueueType};
use num_bigint::BigUint;

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

/// Inserts every element into the indexed array.
/// Throws an error if the element already exists.
/// Expects an indexed queue account as for every index as remaining account.
pub fn process_insert_into_queues<'a, 'b, 'c: 'info, 'info, MerkleTreeAccount: Owner + ZeroCopy>(
    ctx: Context<'a, 'b, 'c, 'info, InsertIntoQueues<'info>>,
    elements: &'a [[u8; 32]],
    indices: Vec<u32>,
    queue_type: QueueType,
    tx_hash: Option<[u8; 32]>,
) -> Result<()> {
    if elements.is_empty() {
        return err!(AccountCompressionErrorCode::InputElementsEmpty);
    }

    light_heap::bench_sbf_start!("acp_create_queue_map");

    let mut queue_map = QueueMap::new();
    // Deduplicate tree and queue pairs.
    // So that we iterate over every pair only once,
    // and pay rollover fees only once.
    let mut current_index = 0;
    for (index, element) in elements.iter().enumerate() {
        let current_account_discriminator = ctx
            .remaining_accounts
            .get(current_index)
            .unwrap()
            .try_borrow_data()?[0..8]
            .try_into()
            .unwrap();
        match current_account_discriminator {
            QueueAccount::DISCRIMINATOR => add_queue_bundle_v1(
                &mut current_index,
                queue_type,
                &mut queue_map,
                element,
                ctx.remaining_accounts,
            )?,
            BatchedQueueAccount::DISCRIMINATOR => add_queue_bundle_v2(
                &mut current_index,
                queue_type,
                &mut queue_map,
                element,
                indices[index],
                ctx.remaining_accounts,
            )?,
            // Address queue is part of the address Merkle tree account.
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
        let rollover_fee = match queue_bundle.queue_type {
            QueueType::NullifierQueue => process_queue_bundle_v1(&ctx, queue_bundle),
            QueueType::AddressQueue => process_queue_bundle_v1(&ctx, queue_bundle),
            QueueType::Input => process_queue_bundle_v2(&ctx, queue_bundle, &tx_hash),
            QueueType::Address => process_address_queue_bundle_v2(&ctx, queue_bundle),
            _ => {
                msg!("Queue type {:?} is not supported", queue_bundle.queue_type);
                return err!(AccountCompressionErrorCode::InvalidQueueType);
            }
        }?;

        if rollover_fee > 0 {
            transfer_lamports_cpi(
                &ctx.accounts.fee_payer,
                // Queue account
                &queue_bundle.accounts[0].to_account_info(),
                rollover_fee,
            )?;
        }
    }

    Ok(())
}

fn process_queue_bundle_v1<'info>(
    ctx: &Context<'_, '_, '_, 'info, InsertIntoQueues<'info>>,
    queue_bundle: &QueueBundle<'_, '_>,
) -> Result<u64> {
    let queue = AccountLoader::<QueueAccount>::try_from(queue_bundle.accounts[0])?;
    light_heap::bench_sbf_start!("acp_prep_insertion");
    let rollover_fee = {
        let queue = queue.load()?;
        check_signer_is_registered_or_authority::<InsertIntoQueues, QueueAccount>(ctx, &queue)?;

        queue.metadata.rollover_metadata.rollover_fee * queue_bundle.elements.len() as u64
    };
    {
        let sequence_number = {
            let merkle_tree = queue_bundle.accounts[1].try_borrow_data()?;
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
    Ok(rollover_fee)
}

fn process_queue_bundle_v2<'info>(
    ctx: &Context<'_, '_, '_, 'info, InsertIntoQueues<'info>>,
    queue_bundle: &QueueBundle<'_, '_>,
    tx_hash: &Option<[u8; 32]>,
) -> Result<u64> {
    let merkle_tree = &mut ZeroCopyBatchedMerkleTreeAccount::state_tree_from_account_info_mut(
        queue_bundle.accounts[0],
    )
    .map_err(ProgramError::from)?;

    let output_queue = &mut ZeroCopyBatchedQueueAccount::output_queue_from_account_info_mut(
        queue_bundle.accounts[1],
    )
    .map_err(ProgramError::from)?;
    check_signer_is_registered_or_authority::<InsertIntoQueues, ZeroCopyBatchedMerkleTreeAccount>(
        ctx,
        merkle_tree,
    )?;
    let rollover_fee = merkle_tree
        .get_account()
        .metadata
        .rollover_metadata
        .rollover_fee
        * queue_bundle.elements.len() as u64;
    for (element, leaf_index) in queue_bundle
        .elements
        .iter()
        .zip(queue_bundle.indices.iter())
    {
        let tx_hash = tx_hash.ok_or(AccountCompressionErrorCode::TxHashUndefined)?;
        light_heap::bench_sbf_start!("acp_insert_nf_into_queue_v2");
        // check for every account whether the value is still in the queue and zero it out.
        // If checked fail if the value is not in the queue.
        output_queue
            .prove_inclusion_by_index_and_zero_out_leaf(*leaf_index as u64, element)
            .map_err(ProgramError::from)?;
        merkle_tree
            .insert_nullifier_into_current_batch(element, *leaf_index as u64, &tx_hash)
            .map_err(ProgramError::from)?;
        light_heap::bench_sbf_end!("acp_insert_nf_into_queue_v2");
    }
    Ok(rollover_fee)
}

fn process_address_queue_bundle_v2<'info>(
    ctx: &Context<'_, '_, '_, 'info, InsertIntoQueues<'info>>,
    queue_bundle: &QueueBundle<'_, '_>,
) -> Result<u64> {
    let merkle_tree = &mut ZeroCopyBatchedMerkleTreeAccount::address_tree_from_account_info_mut(
        queue_bundle.accounts[0],
    )
    .map_err(ProgramError::from)?;
    check_signer_is_registered_or_authority::<InsertIntoQueues, ZeroCopyBatchedMerkleTreeAccount>(
        ctx,
        merkle_tree,
    )?;
    let rollover_fee = merkle_tree
        .get_account()
        .metadata
        .rollover_metadata
        .rollover_fee
        * queue_bundle.elements.len() as u64;
    for element in queue_bundle.elements.iter() {
        light_heap::bench_sbf_start!("acp_insert_nf_into_queue_v2");
        merkle_tree
            .insert_address_into_current_batch(element)
            .map_err(ProgramError::from)?;
        light_heap::bench_sbf_end!("acp_insert_nf_into_queue_v2");
    }
    Ok(rollover_fee)
}

fn add_queue_bundle_v1<'a, 'info>(
    remaining_accounts_index: &mut usize,
    queue_type: QueueType,
    queue_map: &mut std::collections::HashMap<Pubkey, QueueBundle<'a, 'info>>,
    element: &'a [u8; 32],
    remaining_accounts: &'info [AccountInfo<'info>],
) -> Result<()> {
    let queue = remaining_accounts.get(*remaining_accounts_index).unwrap();
    let merkle_tree = remaining_accounts
        .get(*remaining_accounts_index + 1)
        .unwrap();
    let associated_merkle_tree = {
        let queue = AccountLoader::<QueueAccount>::try_from(queue)?;
        let queue = queue.load()?;
        check_queue_type(&queue.metadata.queue_type, &queue_type).map_err(ProgramError::from)?;
        queue.metadata.associated_merkle_tree
    };
    if merkle_tree.key() != associated_merkle_tree {
        msg!(
            "Queue account {:?} is not associated with Merkle tree  {:?}",
            queue.key(),
            merkle_tree.key()
        );
        return err!(AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated);
    }
    queue_map
        .entry(queue.key())
        .or_insert_with(|| QueueBundle::new(queue_type, vec![queue, merkle_tree]))
        .elements
        .push(element);
    *remaining_accounts_index += 2;
    Ok(())
}

fn add_queue_bundle_v2<'a, 'info>(
    remaining_accounts_index: &mut usize,
    queue_type: QueueType,
    queue_map: &mut std::collections::HashMap<Pubkey, QueueBundle<'a, 'info>>,
    element: &'a [u8; 32],
    index: u32,
    remaining_accounts: &'info [AccountInfo<'info>],
) -> Result<()> {
    if queue_type != QueueType::NullifierQueue {
        msg!("Queue type Address is not supported for BatchedMerkleTreeAccount");
        return err!(AccountCompressionErrorCode::InvalidQueueType);
    }
    let output_queue = remaining_accounts.get(*remaining_accounts_index).unwrap();
    let merkle_tree = remaining_accounts
        .get(*remaining_accounts_index + 1)
        .unwrap();
    let output_queue_account =
        ZeroCopyBatchedQueueAccount::output_queue_from_account_info_mut(output_queue)
            .map_err(ProgramError::from)?;
    let associated_merkle_tree = output_queue_account
        .get_account()
        .metadata
        .associated_merkle_tree;

    if merkle_tree.key() != associated_merkle_tree {
        msg!(
            "Queue account {:?} is not associated with Merkle tree {:?}",
            output_queue.key(),
            merkle_tree.key()
        );
        return err!(AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated);
    }
    queue_map
        .entry(merkle_tree.key())
        .or_insert_with(|| QueueBundle::new(QueueType::Input, vec![merkle_tree, output_queue]))
        .elements
        .push(element);
    queue_map
        .entry(merkle_tree.key())
        .and_modify(|x| x.indices.push(index));
    *remaining_accounts_index += 2;

    Ok(())
}

fn add_address_queue_bundle_v2<'a, 'info>(
    remaining_accounts_index: &mut usize,
    queue_type: QueueType,
    queue_map: &mut std::collections::HashMap<Pubkey, QueueBundle<'a, 'info>>,
    address: &'a [u8; 32],
    remaining_accounts: &'info [AccountInfo<'info>],
) -> Result<()> {
    if queue_type != QueueType::AddressQueue {
        msg!("Queue type Address is not supported for BatchedMerkleTreeAccount");
        return err!(AccountCompressionErrorCode::InvalidQueueType);
    }
    let merkle_tree = remaining_accounts.get(*remaining_accounts_index).unwrap();

    // TODO: Reconsider whether we can avoid sending the Merkle tree account
    // twice. Right now we do it for conistency with usage for other by
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
    queue_map
        .entry(merkle_tree.key())
        .or_insert_with(|| QueueBundle::new(QueueType::Address, vec![merkle_tree]))
        .elements
        .push(address);
    *remaining_accounts_index += 2;

    Ok(())
}

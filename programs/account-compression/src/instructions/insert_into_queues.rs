use crate::{
    batched_merkle_tree::{BatchedMerkleTreeAccount, ZeroCopyBatchedMerkleTreeAccount},
    check_queue_type,
    errors::AccountCompressionErrorCode,
    state::queue::{queue_from_bytes_zero_copy_mut, QueueAccount},
    state_merkle_tree_from_bytes_zero_copy,
    utils::{
        check_signer_is_registered_or_authority::check_signer_is_registered_or_authority,
        queue::{QueueBundle, QueueMap},
        transfer_lamports::transfer_lamports_cpi,
    },
    QueueType, RegisteredProgram,
};
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey, Discriminator, ZeroCopy};
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

// TODO: refactor and add rust native unit tests
/// Inserts every element into the indexed array.
/// Throws an error if the element already exists.
/// Expects an indexed queue account as for every index as remaining account.
pub fn process_insert_into_queues<'a, 'b, 'c: 'info, 'info, MerkleTreeAccount: Owner + ZeroCopy>(
    ctx: Context<'a, 'b, 'c, 'info, InsertIntoQueues<'info>>,
    elements: &'a [[u8; 32]],
    queue_type: QueueType,
) -> Result<()> {
    // TODO: pass tx hash with instruction data
    let tx_hash = [0u8; 32];

    if elements.is_empty() {
        return err!(AccountCompressionErrorCode::InputElementsEmpty);
    }

    light_heap::bench_sbf_start!("acp_create_queue_map");

    let mut queue_map = QueueMap::new();
    // Deduplicate tree and queue pairs.
    // So that we iterate over every pair only once,
    // and pay rollover fees only once.
    let mut current_index = 0;
    for element in elements.iter() {
        // TODO: remove unwrap
        let current_account_discriminator = ctx
            .remaining_accounts
            .get(current_index)
            .unwrap()
            .try_borrow_data()?[0..8]
            .try_into()
            .unwrap();
        match current_account_discriminator {
            QueueAccount::DISCRIMINATOR => add_queue_bundle_v0(
                &mut current_index,
                queue_type,
                &mut queue_map,
                element,
                ctx.remaining_accounts,
            )?,
            BatchedMerkleTreeAccount::DISCRIMINATOR => add_queue_bundle_v1(
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
                return err!(AccountCompressionErrorCode::InvalidDiscriminator);
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
        // TODO: match queue bundle type here
        let rollover_fee = match queue_bundle.queue_type {
            QueueType::NullifierQueue => process_queue_bundle_v0(&ctx, queue_bundle),
            QueueType::AddressQueue => process_queue_bundle_v0(&ctx, queue_bundle),
            QueueType::Input => process_queue_bundle_v1(&ctx, queue_bundle, &tx_hash),
            _ => {
                msg!("Queue type {:?} is not supported", queue_bundle.queue_type);
                return err!(AccountCompressionErrorCode::InvalidQueueType);
            }
        }?;

        if rollover_fee > 0 {
            transfer_lamports_cpi(
                &ctx.accounts.fee_payer,
                // Queue account
                &queue_bundle.accounts[1].to_account_info(),
                rollover_fee,
            )?;
        }
    }

    Ok(())
}

fn process_queue_bundle_v0<'info>(
    ctx: &Context<'_, '_, '_, 'info, InsertIntoQueues<'info>>,
    queue_bundle: &QueueBundle<'_, '_>,
) -> Result<u64> {
    let queue = AccountLoader::<QueueAccount>::try_from(queue_bundle.accounts[1])?;
    light_heap::bench_sbf_start!("acp_prep_insertion");
    let rollover_fee = {
        let queue = queue.load()?;
        check_signer_is_registered_or_authority::<InsertIntoQueues, QueueAccount>(ctx, &queue)?;

        queue.metadata.rollover_metadata.rollover_fee * queue_bundle.elements.len() as u64
    };
    {
        let sequence_number = {
            let merkle_tree = queue_bundle.accounts[0].try_borrow_data()?;
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

fn process_queue_bundle_v1<'info>(
    ctx: &Context<'_, '_, '_, 'info, InsertIntoQueues<'info>>,
    queue_bundle: &QueueBundle<'_, '_>,
    tx_hash: &[u8; 32],
) -> Result<u64> {
    msg!("Processing queue bundle v1");
    let account_data = &mut queue_bundle.accounts[0].try_borrow_mut_data()?;
    let merkle_tree = &mut ZeroCopyBatchedMerkleTreeAccount::from_bytes_mut(account_data)?;
    msg!("Checking signer");
    check_signer_is_registered_or_authority::<InsertIntoQueues, ZeroCopyBatchedMerkleTreeAccount>(
        ctx,
        merkle_tree,
    )?;
    msg!("Checking rollover fee");
    let rollover_fee = merkle_tree
        .get_account()
        .metadata
        .rollover_metadata
        .rollover_fee
        * queue_bundle.elements.len() as u64;
    for element in queue_bundle.elements.iter() {
        msg!("element {:?}", element);
        light_heap::bench_sbf_start!("acp_insert_nf_into_queue_v1");
        merkle_tree.insert_nullifier_into_current_batch(element, tx_hash)?;
        light_heap::bench_sbf_end!("acp_insert_nf_into_queue_v1");
    }
    Ok(rollover_fee)
}

fn add_queue_bundle_v0<'a, 'info>(
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
        check_queue_type(&queue.metadata.queue_type, &queue_type)?;
        queue.metadata.associated_merkle_tree
    };
    if merkle_tree.key() != associated_merkle_tree {
        msg!(
                "Queue account {:?} is not associated with any address Merkle tree. Provided accounts {:?}",
                queue.key(), remaining_accounts);
        return err!(AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated);
    }
    queue_map
        .entry(queue.key())
        .or_insert_with(|| QueueBundle::new(queue_type, vec![merkle_tree, queue]))
        .elements
        .push(element);
    *remaining_accounts_index += 2;
    Ok(())
}

fn add_queue_bundle_v1<'a, 'info>(
    remaining_accounts_index: &mut usize,
    queue_type: QueueType,
    queue_map: &mut std::collections::HashMap<Pubkey, QueueBundle<'a, 'info>>,
    element: &'a [u8; 32],
    remaining_accounts: &'info [AccountInfo<'info>],
) -> Result<()> {
    // TODO: add address support
    if queue_type == QueueType::Address {
        msg!("Queue type Address is not supported for BatchedMerkleTreeAccount");
        return err!(AccountCompressionErrorCode::InvalidQueueType);
    }
    let merkle_tree = remaining_accounts.get(*remaining_accounts_index).unwrap();
    queue_map
        .entry(merkle_tree.key())
        .or_insert_with(|| QueueBundle::new(QueueType::Input, vec![merkle_tree]))
        .elements
        .push(element);
    *remaining_accounts_index += 1;

    Ok(())
}

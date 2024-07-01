use crate::{
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
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey, ZeroCopy};
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
    queue_type: QueueType,
) -> Result<()> {
    if elements.is_empty() {
        return err!(AccountCompressionErrorCode::InputElementsEmpty);
    }
    let expected_remaining_accounts = elements.len() * 2;
    if expected_remaining_accounts != ctx.remaining_accounts.len() {
        msg!(
            "Number of remaining accounts does not match, expected {}, got {}",
            expected_remaining_accounts,
            ctx.remaining_accounts.len()
        );
        return err!(crate::errors::AccountCompressionErrorCode::NumberOfLeavesMismatch);
    }
    light_heap::bench_sbf_start!("acp_create_queue_map");

    let mut queue_map = QueueMap::new();
    // Deduplicate tree and queue pairs.
    // So that we iterate over every pair only once,
    // and pay rollover fees only once.
    for i in (0..ctx.remaining_accounts.len()).step_by(2) {
        let queue: &AccountInfo<'info> = ctx.remaining_accounts.get(i).unwrap();
        let merkle_tree = ctx.remaining_accounts.get(i + 1).unwrap();
        let associated_merkle_tree = {
            let queue = AccountLoader::<QueueAccount>::try_from(queue)?;
            let queue = queue.load()?;
            check_queue_type(&queue.metadata.queue_type, &queue_type)?;
            queue.metadata.associated_merkle_tree
        };

        if merkle_tree.key() != associated_merkle_tree {
            msg!(
                    "Queue account {:?} is not associated with any address Merkle tree. Provided accounts {:?}",
                    queue.key(), ctx.remaining_accounts);
            return err!(AccountCompressionErrorCode::MerkleTreeAndQueueNotAssociated);
        }

        queue_map
            .entry(queue.key())
            .or_insert_with(|| QueueBundle::new(queue, merkle_tree))
            .elements
            .push(elements[i / 2]);
    }

    light_heap::bench_sbf_end!("acp_create_queue_map");

    for queue_bundle in queue_map.values() {
        let rollover_fee: u64;

        let queue = AccountLoader::<QueueAccount>::try_from(queue_bundle.queue)?;
        light_heap::bench_sbf_start!("acp_prep_insertion");
        {
            let queue = queue.load()?;
            check_signer_is_registered_or_authority::<InsertIntoQueues, QueueAccount>(
                &ctx, &queue,
            )?;
            rollover_fee =
                queue.metadata.rollover_metadata.rollover_fee * queue_bundle.elements.len() as u64;
        }
        {
            let sequence_number = {
                let merkle_tree = queue_bundle.merkle_tree.try_borrow_data()?;
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

        if rollover_fee > 0 {
            transfer_lamports_cpi(
                &ctx.accounts.fee_payer,
                &queue_bundle.queue.to_account_info(),
                rollover_fee,
            )?;
        }
    }

    Ok(())
}

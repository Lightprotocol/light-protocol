use crate::{
    check_queue_type,
    errors::AccountCompressionErrorCode,
    state::queue::{queue_from_bytes_zero_copy_mut, QueueAccount},
    utils::{
        check_signer_is_registered_or_authority::check_signer_is_registered_or_authority,
        queue::{QueueBundle, QueueMap},
        transfer_lamports::transfer_lamports_cpi,
    },
    QueueType, RegisteredProgram, SequenceNumber,
};
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey, ZeroCopy};
use num_bigint::BigUint;

#[derive(Accounts)]
pub struct InsertIntoQueues<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK: should only be accessed by a registered program/owner/program_owner.
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>, // nullifiers are sent in remaining accounts. @ErrorCode::InvalidVerifier
    pub system_program: Program<'info, System>,
}
// TODO: add test that the first two elements of the queue cannot be inserted into the tree and removed from the queue
/// Inserts every element into the indexed array.
/// Throws an error if the element already exists.
/// Expects an indexed queue account as for every index as remaining account.
pub fn process_insert_into_queues<
    'a,
    'b,
    'c: 'info,
    'info,
    MerkleTreeAccount: Owner + ZeroCopy + SequenceNumber,
>(
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
        let lamports: u64;

        let indexed_array = AccountLoader::<QueueAccount>::try_from(queue_bundle.queue)?;
        light_heap::bench_sbf_start!("acp_prep_insertion");
        {
            let indexed_array = indexed_array.load()?;
            check_signer_is_registered_or_authority::<InsertIntoQueues, QueueAccount>(
                &ctx,
                &indexed_array,
            )?;
            if queue_bundle.merkle_tree.key() != indexed_array.metadata.associated_merkle_tree {
                return err!(AccountCompressionErrorCode::InvalidMerkleTree);
            }
            lamports = indexed_array.metadata.rollover_metadata.rollover_fee
                * queue_bundle.elements.len() as u64;
        }
        {
            let merkle_tree =
                AccountLoader::<MerkleTreeAccount>::try_from(queue_bundle.merkle_tree)?;
            let sequence_number = merkle_tree.load()?.get_sequence_number()?;

            let indexed_array = indexed_array.to_account_info();
            let mut indexed_array = indexed_array.try_borrow_mut_data()?;
            let mut indexed_array =
                unsafe { queue_from_bytes_zero_copy_mut(&mut indexed_array).unwrap() };
            light_heap::bench_sbf_end!("acp_prep_insertion");
            light_heap::bench_sbf_start!("acp_insert_nf_into_queue");
            for element in queue_bundle.elements.iter() {
                let element = BigUint::from_bytes_be(element.as_slice());
                indexed_array
                    .insert(&element, sequence_number)
                    .map_err(ProgramError::from)?;
            }
            light_heap::bench_sbf_end!("acp_insert_nf_into_queue");
        }

        if lamports > 0 {
            transfer_lamports_cpi(
                &ctx.accounts.fee_payer,
                &queue_bundle.queue.to_account_info(),
                lamports,
            )?;
        }
    }

    Ok(())
}

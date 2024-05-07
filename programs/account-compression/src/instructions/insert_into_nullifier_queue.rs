use crate::{
    errors::AccountCompressionErrorCode,
    processor::initialize_nullifier_queue::{
        nullifier_queue_from_bytes_zero_copy_mut, NullifierQueueAccount,
    },
    transfer_lamports_cpi,
    utils::{
        check_registered_or_signer::check_registered_or_signer,
        queue::{QueueBundle, QueueMap},
    },
    RegisteredProgram, StateMerkleTreeAccount,
};
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
use light_heap::BumpAllocator;
use num_bigint::BigUint;

#[derive(Accounts)]
pub struct InsertIntoNullifierQueues<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    /// CHECK: should only be accessed by a registered program/owner/delegate.
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>, // nullifiers are sent in remaining accounts. @ErrorCode::InvalidVerifier
    pub system_program: Program<'info, System>,
}

/// Inserts every element into the indexed array.
/// Throws an error if the element already exists.
/// Expects an indexed queue account as for every index as remaining account.
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
        return err!(crate::errors::AccountCompressionErrorCode::NumberOfLeavesMismatch);
    }
    light_heap::bench_sbf_start!("acp_create_queue_map");

    let mut queue_map = QueueMap::new();
    for i in 0..elements.len() {
        let queue = ctx.remaining_accounts.get(i).unwrap();
        let merkle_tree = ctx.remaining_accounts.get(elements.len() + i).unwrap();
        let unpacked_queue_account =
            AccountLoader::<NullifierQueueAccount>::try_from(queue).unwrap();
        let array_account = unpacked_queue_account.load()?;

        if array_account.associated_merkle_tree != merkle_tree.key() {
            msg!(
                "Nullifier queue account {:?} is not associated with any state Merkle tree {:?}. Associated State Merkle tree {:?}",
               queue.key() ,merkle_tree.key(), array_account.associated_merkle_tree);
            return Err(AccountCompressionErrorCode::InvalidNullifierQueue.into());
        }

        queue_map
            .entry(queue.key())
            .or_insert_with(|| QueueBundle::new(queue, merkle_tree))
            .elements
            .push(elements[i]);
    }
    light_heap::bench_sbf_end!("acp_create_queue_map");

    for queue_bundle in queue_map.values() {
        let lamports: u64;

        let indexed_array = AccountLoader::<NullifierQueueAccount>::try_from(queue_bundle.queue)?;
        light_heap::bench_sbf_start!("acp_prep_insertion");
        {
            let indexed_array = indexed_array.load()?;
            check_registered_or_signer::<InsertIntoNullifierQueues, NullifierQueueAccount>(
                &ctx,
                &indexed_array,
            )?;
            if queue_bundle.merkle_tree.key() != indexed_array.associated_merkle_tree {
                return err!(AccountCompressionErrorCode::InvalidMerkleTree);
            }
            lamports =
                indexed_array.tip + indexed_array.rollover_fee * queue_bundle.elements.len() as u64;
        }
        {
            let merkle_tree =
                AccountLoader::<StateMerkleTreeAccount>::try_from(queue_bundle.merkle_tree)?;
            let sequence_number = {
                let merkle_tree = merkle_tree.load()?;
                merkle_tree.load_merkle_tree()?.sequence_number
            };

            let indexed_array = indexed_array.to_account_info();
            let mut indexed_array = indexed_array.try_borrow_mut_data()?;
            let mut indexed_array =
                unsafe { nullifier_queue_from_bytes_zero_copy_mut(&mut indexed_array).unwrap() };
            light_heap::bench_sbf_end!("acp_prep_insertion");
            light_heap::bench_sbf_start!("acp_insert_nf_into_queue");
            for element in queue_bundle.elements.iter() {
                #[cfg(target_os = "solana")]
                let pos = light_heap::GLOBAL_ALLOCATOR.get_heap_pos();
                let element = BigUint::from_bytes_be(element.as_slice());
                msg!("Inserting element {:?} into nullifier queue", element);
                indexed_array
                    .insert(&element, sequence_number)
                    .map_err(ProgramError::from)?;
                #[cfg(target_os = "solana")]
                light_heap::GLOBAL_ALLOCATOR.free_heap(pos);
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

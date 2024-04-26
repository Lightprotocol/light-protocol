use anchor_lang::prelude::*;
use num_bigint::BigUint;

use crate::{
    address_queue_from_bytes_zero_copy_mut,
    errors::AccountCompressionErrorCode,
    transfer_lamports_cpi,
    utils::{
        check_registered_or_signer::{check_registered_or_signer, GroupAccounts},
        queue::{QueueBundle, QueueMap},
    },
    AddressMerkleTreeAccount, AddressQueueAccount, RegisteredProgram,
};

#[derive(Accounts)]
pub struct InsertAddresses<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
    pub system_program: Program<'info, System>,
}

impl<'info> GroupAccounts<'info> for InsertAddresses<'info> {
    fn get_signing_address(&self) -> &Signer<'info> {
        &self.authority
    }

    fn get_registered_program_pda(&self) -> &Option<Account<'info, RegisteredProgram>> {
        &self.registered_program_pda
    }
}

pub fn process_insert_addresses<'a, 'b, 'c: 'info, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, InsertAddresses<'info>>,
    addresses: Vec<[u8; 32]>,
) -> Result<()> {
    let expected_remaining_accounts = addresses.len() * 2;
    if expected_remaining_accounts != ctx.remaining_accounts.len() {
        msg!(
            "Number of remaining accounts does not match, expected {}, got {}",
            expected_remaining_accounts,
            ctx.remaining_accounts.len()
        );
        return err!(AccountCompressionErrorCode::NumberOfLeavesMismatch);
    }
    let mut queue_map = QueueMap::new();

    for i in (0..ctx.remaining_accounts.len()).step_by(2) {
        let queue: &AccountInfo<'info> = ctx.remaining_accounts.get(i).unwrap();
        let associated_merkle_tree = {
            let queue = AccountLoader::<AddressQueueAccount>::try_from(queue)?;
            let queue = queue.load()?;
            queue.associated_merkle_tree
        };

        let merkle_tree = ctx.remaining_accounts.get(i + 1).unwrap();
        if merkle_tree.key() != associated_merkle_tree {
            msg!(
                    "Address queue account {:?} is not associated with any address Merkle tree. Provided accounts {:?}",
                    queue.key(), ctx.remaining_accounts);
            return err!(AccountCompressionErrorCode::InvalidNullifierQueue);
        }

        queue_map
            .entry(queue.key())
            .or_insert_with(|| QueueBundle::new(queue, merkle_tree))
            .elements
            .push(addresses[i / 2]);
    }

    for queue_bundle in queue_map.values() {
        let lamports;
        let address_queue = AccountLoader::<AddressQueueAccount>::try_from(queue_bundle.queue)?;
        {
            let address_queue = address_queue.load()?;
            check_registered_or_signer::<InsertAddresses, AddressQueueAccount>(
                &ctx,
                &address_queue,
            )?;
            if queue_bundle.merkle_tree.key() != address_queue.associated_merkle_tree {
                return err!(AccountCompressionErrorCode::InvalidMerkleTree);
            }
            lamports =
                address_queue.tip + address_queue.rollover_fee * queue_bundle.elements.len() as u64;
            drop(address_queue);
        }

        {
            let merkle_tree =
                AccountLoader::<AddressMerkleTreeAccount>::try_from(queue_bundle.merkle_tree)?;
            let sequence_number = {
                let merkle_tree = merkle_tree.load()?;
                merkle_tree.load_merkle_tree()?.merkle_tree.sequence_number
            };

            let address_queue = address_queue.to_account_info();
            let mut address_queue = address_queue.try_borrow_mut_data()?;
            let mut address_queue =
                unsafe { address_queue_from_bytes_zero_copy_mut(&mut address_queue)? };

            for address in queue_bundle.elements.iter() {
                msg!("Inserting address {:?}", address);
                let address = BigUint::from_bytes_be(address.as_slice());
                address_queue
                    .insert(&address, sequence_number)
                    .map_err(ProgramError::from)?;
            }
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

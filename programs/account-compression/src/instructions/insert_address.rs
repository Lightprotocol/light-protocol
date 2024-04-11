use anchor_lang::prelude::*;
use num_bigint::BigUint;

use crate::{
    address_queue_from_bytes_zero_copy_mut,
    errors::AccountCompressionErrorCode,
    utils::{
        check_registered_or_signer::{check_registered_or_signer, GroupAccounts},
        queue::{QueueBundle, QueueMap},
    },
    AddressMerkleTreeAccount, AddressQueueAccount, RegisteredProgram,
};

#[derive(Accounts)]
pub struct InsertAddresses<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    pub registered_program_pda: Option<Account<'info, RegisteredProgram>>,
}

impl<'info> GroupAccounts<'info> for InsertAddresses<'info> {
    fn get_signing_address(&self) -> &Signer<'info> {
        &self.authority
    }

    fn get_registered_program_pda(&self) -> &Option<Account<'info, crate::RegisteredProgram>> {
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
    for i in 0..addresses.len() {
        let queue = ctx.remaining_accounts.get(i).unwrap();
        let associated_merkle_tree = {
            let queue = AccountLoader::<AddressQueueAccount>::try_from(queue)?;
            let queue = queue.load()?;
            queue.associated_merkle_tree
        };

        let mut merkle_tree = None;
        for j in 0..addresses.len() {
            let merkle_tree_candidate = ctx.remaining_accounts.get(addresses.len() + j).unwrap();
            if merkle_tree_candidate.key() == associated_merkle_tree {
                merkle_tree = Some(merkle_tree_candidate);
            }
        }

        let merkle_tree = match merkle_tree {
            Some(merkle_tree) => merkle_tree,
            None => {
                msg!(
                    "Address queue account {:?} is not associated with any address Merkle tree. Provided accounts {:?}",
                    queue.key(), ctx.remaining_accounts);
                return err!(AccountCompressionErrorCode::InvalidIndexedArray);
            }
        };

        queue_map
            .entry(queue.key())
            .or_insert_with(|| QueueBundle::new(queue, merkle_tree))
            .elements
            .push(addresses[i]);
    }

    for queue_bundle in queue_map.values() {
        msg!(
            "Inserting into address queue {:?}",
            queue_bundle.queue.key()
        );

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
            drop(address_queue);
        }

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

    Ok(())
}

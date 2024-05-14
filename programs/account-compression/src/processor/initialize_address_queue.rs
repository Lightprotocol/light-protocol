use anchor_lang::prelude::*;
use light_utils::fee::compute_rollover_fee;

use crate::{address_queue_from_bytes_zero_copy_init, state::AddressQueueAccount};

#[derive(Accounts)]
pub struct InitializeAddressQueue<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(zero)]
    pub queue: AccountLoader<'info, AddressQueueAccount>,
}

pub fn process_initialize_address_queue<'info>(
    queue_account_info: &AccountInfo<'info>,
    queue_loader: &AccountLoader<'info, AddressQueueAccount>,
    index: u64,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    associated_merkle_tree: Pubkey,
    capacity_indices: u16,
    capacity_values: u16,
    sequence_threshold: u64,
    tip: u64,
    rollover_threshold: Option<u64>,
    height: u32,
    merkle_tree_rent: u64,
) -> Result<()> {
    {
        let mut address_queue_account = queue_loader.load_init()?;
        address_queue_account.index = index;
        address_queue_account.owner = owner;
        address_queue_account.delegate = delegate.unwrap_or_default();
        address_queue_account.associated_merkle_tree = associated_merkle_tree;
        address_queue_account.tip = tip;
        // Rollover only makes sense in combination with the associated merkle tree
        let queue_rent = queue_account_info.lamports();
        // Since user doesn't interact with the Merkle tree directly, we need to
        // charge a `rollover_fee` both for the queue and Merkle tree.
        let rollover_fee = if let Some(rollover_threshold) = rollover_threshold {
            compute_rollover_fee(rollover_threshold, height, merkle_tree_rent)
                .map_err(ProgramError::from)?
                + compute_rollover_fee(rollover_threshold, height, queue_rent)
                    .map_err(ProgramError::from)?
        } else {
            0
        };

        address_queue_account.rolledover_slot = u64::MAX;
        address_queue_account.rollover_fee = rollover_fee;
        drop(address_queue_account);
    }

    let _ = unsafe {
        address_queue_from_bytes_zero_copy_init(
            &mut queue_account_info.try_borrow_mut_data()?,
            capacity_indices as usize,
            capacity_values as usize,
            sequence_threshold as usize,
        )
        .unwrap()
    };

    Ok(())
}

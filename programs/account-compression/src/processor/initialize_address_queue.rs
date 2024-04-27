use anchor_lang::prelude::*;

use crate::{
    address_queue_from_bytes_zero_copy_init, initialize_address_merkle_tree::compute_rollover_fee,
    state::AddressQueueAccount,
};

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
    rent: u64,
) -> Result<()> {
    {
        let mut address_queue_account = queue_loader.load_init()?;
        address_queue_account.index = index;
        address_queue_account.owner = owner;
        address_queue_account.delegate = delegate.unwrap_or_default();
        address_queue_account.associated_merkle_tree = associated_merkle_tree;
        address_queue_account.tip = tip;
        // rollover only makes sense in combination with the associated merkle tree
        let total_number_of_leaves = 2u64.pow(height);
        let queue_rent = queue_account_info.lamports();
        let rollover_fee = if let Some(rollover_threshold) = rollover_threshold {
            compute_rollover_fee(rollover_threshold, total_number_of_leaves, rent)?
                + compute_rollover_fee(rollover_threshold, total_number_of_leaves, queue_rent)?
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

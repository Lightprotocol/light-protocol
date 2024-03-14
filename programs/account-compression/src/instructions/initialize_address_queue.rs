use anchor_lang::prelude::*;

use crate::{address_queue_from_bytes_zero_copy_init, state::AddressQueueAccount};

#[derive(Accounts)]
pub struct InitializeAddressQueue<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(zero)]
    pub queue: AccountLoader<'info, AddressQueueAccount>,
}

pub fn process_initialize_address_queue<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeAddressQueue<'info>>,
    capacity_indices: u16,
    capacity_values: u16,
    sequence_threshold: u64,
) -> Result<()> {
    let _ = unsafe {
        address_queue_from_bytes_zero_copy_init(
            ctx.accounts.queue.to_account_info().try_borrow_mut_data()?,
            capacity_indices as usize,
            capacity_values as usize,
            sequence_threshold as usize,
        )
        .unwrap()
    };

    Ok(())
}

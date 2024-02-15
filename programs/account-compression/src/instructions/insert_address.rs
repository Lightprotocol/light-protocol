use account_compression_state::address_queue_from_bytes_mut;
use anchor_lang::prelude::*;
use light_utils::bigint::be_bytes_to_bigint;

use crate::{errors::AccountCompressionErrorCode, AddressQueueAccount};

#[derive(Accounts)]
pub struct InsertAddresses<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub queue: AccountLoader<'info, AddressQueueAccount>,
}

pub fn process_insert_addresses<'info>(
    ctx: Context<'_, '_, '_, 'info, InsertAddresses<'info>>,
    addresses: Vec<[u8; 32]>,
) -> Result<()> {
    let mut address_queue = ctx.accounts.queue.load_mut()?;
    let address_queue = address_queue_from_bytes_mut(&mut address_queue.queue);

    for address in addresses.iter() {
        let address =
            be_bytes_to_bigint(address).map_err(|_| AccountCompressionErrorCode::BytesToBigint)?;
        address_queue
            .append(address)
            .map_err(|_| AccountCompressionErrorCode::AddressQueueInsert)?;
    }

    Ok(())
}

use anchor_lang::prelude::*;
use num_bigint::BigUint;

use crate::{
    address_queue_from_bytes_zero_copy_mut, errors::AccountCompressionErrorCode,
    AddressMerkleTreeAccount, AddressQueueAccount,
};

#[derive(Accounts)]
pub struct InsertAddresses<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub queue: AccountLoader<'info, AddressQueueAccount>,
    #[account(mut)]
    pub merkle_tree: AccountLoader<'info, AddressMerkleTreeAccount>,
}

pub fn process_insert_addresses<'info>(
    ctx: Context<'_, '_, '_, 'info, InsertAddresses<'info>>,
    addresses: Vec<[u8; 32]>,
) -> Result<()> {
    // let address_queue_acc = ctx.accounts.queue.to_account_info();
    // let data =
    //     &mut address_queue_acc.data.borrow_mut()[8 + mem::size_of::<AddressQueueAccount>()..];
    // let address_queue = unsafe { HashSet::<u16>::from_bytes(data) };
    let mut address_queue = unsafe {
        address_queue_from_bytes_zero_copy_mut(
            ctx.accounts.queue.to_account_info().try_borrow_mut_data()?,
        )?
    };

    let merkle_tree = ctx.accounts.merkle_tree.load()?;
    let sequence_number = merkle_tree.load_merkle_tree()?.merkle_tree.sequence_number;

    for address in addresses.iter() {
        let address = BigUint::from_bytes_le(address);
        address_queue
            .insert(&address, sequence_number)
            .map_err(|_| AccountCompressionErrorCode::AddressQueueInsert)?;
    }

    Ok(())
}

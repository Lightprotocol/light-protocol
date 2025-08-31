use anchor_lang::prelude::*;
use light_compressed_account::instruction_data::insert_into_queues::InsertIntoQueuesInstructionData;
use light_zero_copy::traits::ZeroCopyAt;

use super::{
    insert_addresses::insert_addresses, insert_leaves::insert_leaves,
    insert_nullifiers::insert_nullifiers,
};
use crate::{context::AcpAccount, errors::AccountCompressionErrorCode, GenericInstruction};

pub fn process_insert_into_queues<'a, 'b, 'c: 'info, 'info>(
    ctx: &Context<'a, 'b, 'c, 'info, GenericInstruction<'info>>,
    bytes: Vec<u8>,
) -> Result<()> {
    let (inputs, _) = InsertIntoQueuesInstructionData::zero_copy_at(bytes.as_slice())
        .map_err(ProgramError::from)?;
    let authority = ctx.accounts.authority.to_account_info();
    // Checks accounts for every account in remaining accounts:
    // 1. program ownership
    // 2. discriminator
    // 3. signer eligibility
    let mut accounts = AcpAccount::from_account_infos(
        ctx.remaining_accounts,
        &authority,
        inputs.is_invoked_by_program(),
        inputs.bump,
    )?;
    if inputs.nullifiers.is_empty() && inputs.addresses.is_empty() && inputs.leaves.is_empty() {
        return Err(AccountCompressionErrorCode::InputElementsEmpty.into());
    }

    let current_slot = Clock::get()?.slot;
    // msg!("insert_nullifiers {:?}", inputs.nullifiers.len());
    // msg!("insert_leaves {:?}", inputs.leaves.len());
    // msg!("insert_addresses {:?}", inputs.addresses.len());

    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_start!("append_leaves");
    insert_leaves(
        inputs.leaves.as_slice(),
        inputs.start_output_appends,
        inputs.num_output_queues,
        &mut accounts,
        &current_slot,
    )?;
    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_end!("append_leaves");

    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_start!("insert_nullifiers");
    insert_nullifiers(
        inputs.num_queues,
        inputs.tx_hash,
        inputs.nullifiers.as_slice(),
        &mut accounts,
        &current_slot,
    )?;
    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_end!("insert_nullifiers");

    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_start!("insert_addresses");
    insert_addresses(
        inputs.num_address_queues,
        inputs.addresses.as_slice(),
        &mut accounts,
        &current_slot,
    )?;

    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_end!("insert_addresses");
    Ok(())
}

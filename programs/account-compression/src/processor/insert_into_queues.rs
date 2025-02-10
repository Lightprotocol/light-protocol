use anchor_lang::prelude::*;
use light_utils::instruction::insert_into_queues::deserialize_insert_into_queues;

use super::{
    insert_addresses::insert_addresses, insert_leaves::process_append_leaves_to_merkle_trees,
    insert_nullifiers::insert_nullifiers,
};
use crate::{context::LightContext, errors::AccountCompressionErrorCode, GenericInstruction};

pub fn process_insert_into_queues<'a, 'b, 'c: 'info, 'info>(
    ctx: &Context<'a, 'b, 'c, 'info, GenericInstruction<'info>>,
    bytes: Vec<u8>,
) -> Result<()> {
    let authority = ctx.accounts.authority.to_account_info();
    let mut bytes = bytes;
    let inputs = deserialize_insert_into_queues(bytes.as_mut_slice()).unwrap();
    let mut context = LightContext::new(
        &authority,
        ctx.remaining_accounts,
        inputs.is_invoked_by_program(),
        inputs.bump,
    );
    if inputs.nullifiers.is_empty() && inputs.addresses.is_empty() && inputs.leaves.is_empty() {
        return Err(AccountCompressionErrorCode::InputElementsEmpty.into());
    }
    let current_slot = Clock::get()?.slot;
    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_start!("insert_nullifiers");
    insert_nullifiers(
        inputs.num_queues,
        inputs.tx_hash,
        inputs.nullifiers.as_slice(),
        context.remaining_accounts_mut(),
        &current_slot,
    )?;
    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_end!("insert_nullifiers");
    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_start!("append_leaves");
    process_append_leaves_to_merkle_trees(
        inputs.leaves.as_slice(),
        inputs.start_output_appends,
        inputs.num_output_queues,
        context.remaining_accounts_mut(),
        &current_slot,
    )?;
    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_end!("append_leaves");

    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_start!("insert_addresses");
    insert_addresses(
        inputs.num_address_queues,
        inputs.addresses.as_slice(),
        context.remaining_accounts_mut(),
        &current_slot,
    )?;
    #[cfg(feature = "bench-sbf")]
    light_heap::bench_sbf_end!("insert_addresses");
    Ok(())
}

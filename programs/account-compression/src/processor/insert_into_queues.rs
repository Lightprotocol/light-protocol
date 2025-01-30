use anchor_lang::prelude::*;

use super::{
    insert_addresses::insert_addresses, insert_leaves::process_append_leaves_to_merkle_trees,
    insert_nullifiers::insert_nullifiers,
};
use crate::context::LightContext;
use crate::insert_into_queues::deserialize_nullify_append_create_address_inputs;

use crate::GenericInstruction;

pub fn process_insert_into_queues<'a, 'b, 'c: 'info, 'info>(
    ctx: &Context<'a, 'b, 'c, 'info, GenericInstruction<'info>>,
    bytes: Vec<u8>,
) -> Result<()> {
    let fee_payer = ctx.accounts.fee_payer.to_account_info();
    let mut bytes = bytes;
    let inputs = deserialize_nullify_append_create_address_inputs(bytes.as_mut_slice()).unwrap();
    let mut context = LightContext::new(
        ctx.remaining_accounts,
        &fee_payer,
        inputs.is_invoked_by_program(),
        inputs.bump,
    );

    insert_nullifiers(
        inputs.num_queues,
        inputs.tx_hash,
        inputs.nullifiers.as_slice(),
        context.remaining_accounts_mut(),
    )?;

    process_append_leaves_to_merkle_trees(
        inputs.leaves.as_slice(),
        inputs.num_unique_appends,
        context.remaining_accounts_mut(),
    )?;

    insert_addresses(
        inputs.num_address_appends,
        inputs.addresses.as_slice(),
        context.remaining_accounts_mut(),
    )
}

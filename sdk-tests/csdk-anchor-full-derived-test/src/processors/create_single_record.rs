//! Processor for create_single_record instruction.

use anchor_lang::prelude::*;

use crate::d5_markers::{D5RentfreeBare, D5RentfreeBareParams};

/// Process the create_single_record instruction.
/// Called by the instruction handler in the program module.
pub fn process_create_single_record(
    ctx: Context<'_, '_, '_, '_, D5RentfreeBare<'_>>,
    params: D5RentfreeBareParams,
) -> Result<()> {
    let record = &mut ctx.accounts.record;
    record.owner = params.owner;
    record.counter = 0;
    Ok(())
}

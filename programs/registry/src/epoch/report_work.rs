use anchor_lang::prelude::*;

use super::register_epoch::{EpochPda, ForesterEpochPda};
use crate::errors::RegistryError;
/// Report work:
/// - work is reported so that relative performance of foresters can be assessed
/// 1. Check that we are in the report work phase
/// 2. Check that forester has registered for the epoch
/// 3. Check that forester has not already reported work
/// 4. Add work to total work
pub fn report_work_instruction(
    forester_epoch_pda: &mut ForesterEpochPda,
    epoch_pda: &mut EpochPda,
    current_slot: u64,
) -> Result<()> {
    epoch_pda
        .protocol_config
        .is_report_work_phase(current_slot, epoch_pda.epoch)?;

    if forester_epoch_pda.epoch != epoch_pda.epoch {
        return err!(RegistryError::InvalidEpochAccount);
    }
    if forester_epoch_pda.has_reported_work {
        return err!(RegistryError::ForesterAlreadyRegistered);
    }

    forester_epoch_pda.has_reported_work = true;
    epoch_pda.total_work += forester_epoch_pda.work_counter;
    Ok(())
}

#[derive(Accounts)]
pub struct ReportWork<'info> {
    authority: Signer<'info>,
    #[account(mut, has_one = authority)]
    pub forester_epoch_pda: Account<'info, ForesterEpochPda>,
    #[account(mut)]
    pub epoch_pda: Account<'info, EpochPda>,
}

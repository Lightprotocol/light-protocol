use crate::errors::RegistryError;

use super::register_epoch::{EpochPda, ForesterEpochPda};
use anchor_lang::prelude::*;
/// Report work:
/// - work is reported so that performance based rewards can be calculated after
///   the report work phase ends
/// 1. Check that we are in the report work phase
/// 2. Check that forester has registered for the epoch
/// 3. Check that forester has not already reported work
/// 4. Add work to total work
///
/// Considerations:
/// - we could remove this phase:
///     -> con: we would have no performance based rewards
///     -> pro: reduced complexity
/// 1. Design possibilities even without a separate phase:
///   - we could introduce a separate reward just per work performed (uncapped,
///     for weighted cap we need this round, hardcoded cap would work without
///     this round)
///   - reward could be in sol, or light tokens
pub fn process_report_work(
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
        return err!(RegistryError::ForesterAlreadyReportedWork);
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

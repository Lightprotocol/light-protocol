use anchor_lang::prelude::*;

use crate::{EpochPda, ForesterEpochPda};

#[derive(Accounts)]
pub struct FinalizeRegistration<'info> {
    pub authority: Signer<'info>,
    #[account(mut,has_one = authority)]
    pub forester_epoch_pda: Account<'info, ForesterEpochPda>,
    /// CHECK: instruction checks that the epoch is the current epoch.
    #[account(constraint = epoch_pda.epoch == forester_epoch_pda.epoch)]
    pub epoch_pda: Account<'info, EpochPda>,
}

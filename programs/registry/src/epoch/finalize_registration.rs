use anchor_lang::prelude::*;

use crate::{EpochPda, ForesterEpochPda};

#[derive(Accounts)]
pub struct FinalizeRegistration<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:
    #[account(mut,has_one = authority)]
    pub forester_epoch_pda: Account<'info, ForesterEpochPda>,
    /// CHECK: TODO: check that this is the correct epoch account
    pub epoch_pda: Account<'info, EpochPda>,
}

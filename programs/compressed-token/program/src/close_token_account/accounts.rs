use anchor_lang::solana_program::program_error::ProgramError;
use light_profiler::profile;
use pinocchio::account_info::AccountInfo;

use crate::shared::AccountIterator;

pub struct CloseTokenAccountAccounts<'info> {
    pub token_account: &'info AccountInfo,
    pub destination: &'info AccountInfo,
    pub authority: &'info AccountInfo,
}

impl<'info> CloseTokenAccountAccounts<'info> {
    #[profile]
    #[inline(always)]
    pub fn validate_and_parse(accounts: &'info [AccountInfo]) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);
        Ok(CloseTokenAccountAccounts {
            token_account: iter.next_mut("token_account")?,
            destination: iter.next_mut("destination")?,
            authority: iter.next_signer("authority")?,
        })
    }
}

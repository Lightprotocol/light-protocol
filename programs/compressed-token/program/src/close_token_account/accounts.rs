use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::check_owner;
use light_ctoken_types::COMPRESSIBLE_TOKEN_ACCOUNT_SIZE;
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;

use crate::{shared::AccountIterator, LIGHT_CPI_SIGNER};

pub struct CloseTokenAccountAccounts<'info> {
    pub token_account: &'info AccountInfo,
    pub destination: &'info AccountInfo,
    pub authority: &'info AccountInfo,
    pub rent_sponsor: Option<&'info AccountInfo>,
}

impl<'info> CloseTokenAccountAccounts<'info> {
    #[profile]
    #[inline(always)]
    pub fn validate_and_parse(accounts: &'info [AccountInfo]) -> Result<Self, ProgramError> {
        let mut iter = AccountIterator::new(accounts);
        let token_account = iter.next_mut("token_account")?;
        check_owner(&LIGHT_CPI_SIGNER.program_id, token_account)?;
        if token_account.data_len() != 165
            && token_account.data_len() != COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize
        {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(CloseTokenAccountAccounts {
            token_account,
            destination: iter.next_mut("destination")?,
            authority: iter.next_signer("authority")?,
            rent_sponsor: if accounts.len() >= 4 {
                Some(iter.next_mut("rent_sponsor")?)
            } else {
                None
            },
        })
    }
}

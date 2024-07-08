use anchor_lang::prelude::*;
use anchor_lang::solana_program::{account_info::AccountInfo, msg, rent::Rent};

use crate::errors::AccountCompressionErrorCode;

/// Checks that the account balance is equal to rent exemption.
pub fn check_account_balance_is_rent_exempt(account_info: &AccountInfo) -> Result<u64> {
    let lamports = account_info.lamports();
    let rent_exemption = (Rent::get()?).minimum_balance(account_info.data_len());
    if lamports != rent_exemption {
        msg!(
            "Account {:?} lamports is not equal to rentexemption: {} != {}",
            account_info.key(),
            lamports,
            rent_exemption
        );
        return err!(AccountCompressionErrorCode::InvalidAccountBalance);
    }
    Ok(lamports)
}

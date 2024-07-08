use anchor_lang::prelude::*;
use anchor_lang::solana_program::{account_info::AccountInfo, msg, rent::Rent};

use crate::errors::AccountCompressionErrorCode;

pub fn check_account_balance_is_rent_exempt(account: &AccountInfo) -> Result<u64> {
    let lamports = account.lamports();
    let rent_exemption = (Rent::get()?).minimum_balance(account.data_len());
    if lamports != rent_exemption {
        msg!(
            "Account lamports is not equal to rentexemption: {} != {}",
            lamports,
            rent_exemption
        );
        return err!(AccountCompressionErrorCode::InvalidAccountBalance);
    }
    Ok(lamports)
}

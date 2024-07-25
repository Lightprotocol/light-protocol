use crate::errors::AccountCompressionErrorCode;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{account_info::AccountInfo, msg, rent::Rent};

/// Checks that the account balance is equal to rent exemption.
pub fn check_account_balance_is_rent_exempt(
    account_info: &AccountInfo,
    expected_size: usize,
) -> Result<u64> {
    let account_size = account_info.data_len();
    if account_size != expected_size {
        msg!(
            "Account {:?} size not equal to expected size. size: {}, expected size {}",
            account_info.key(),
            account_size,
            expected_size
        );
        return err!(AccountCompressionErrorCode::InvalidAccountSize);
    }
    let lamports = account_info.lamports();
    let rent_exemption = (Rent::get()?).minimum_balance(expected_size);
    if lamports != rent_exemption {
        msg!(
            "Account {:?} lamports is not equal to rentexemption: lamports {}, rent exemption {}",
            account_info.key(),
            lamports,
            rent_exemption
        );
        return err!(AccountCompressionErrorCode::InvalidAccountBalance);
    }
    Ok(lamports)
}

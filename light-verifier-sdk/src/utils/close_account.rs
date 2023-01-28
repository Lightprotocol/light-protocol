use crate::errors::VerifierSdkError;
use anchor_lang::prelude::*;
use std::ops::DerefMut;
pub fn close_account(account: &AccountInfo, dest_account: &AccountInfo) -> Result<()> {
    //close account by draining lamports
    let dest_starting_lamports = dest_account.lamports();
    **dest_account.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(account.lamports())
        .ok_or(VerifierSdkError::CloseAccountFailed)?;
    **account.lamports.borrow_mut() = 0;
    let mut data = account.try_borrow_mut_data()?;
    for byte in data.deref_mut().iter_mut() {
        *byte = 0;
    }
    Ok(())
}

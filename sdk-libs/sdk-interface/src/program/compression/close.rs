use light_account_checks::AccountInfoTrait;

use crate::error::{LightPdaError, Result};
// TODO: remove and use directly from light-account-checks
/// Close a native Solana account by transferring lamports and clearing data.
pub fn close<AI: AccountInfoTrait>(info: &AI, sol_destination: &AI) -> Result<()> {
    light_account_checks::close_account(info, sol_destination).map_err(LightPdaError::AccountCheck)
}

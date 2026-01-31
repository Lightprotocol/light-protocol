use crate::{account_info::account_info_trait::AccountInfoTrait, error::AccountError};

/// Close a native Solana account by transferring lamports and clearing data.
///
/// Transfers all lamports to `sol_destination`, assigns the account to the
/// system program (all-zero owner), and resizes data to 0.
///
/// If `info` and `sol_destination` are the same account, the lamports stay
/// in the account but the owner and data are cleared.
pub fn close_account<AI: AccountInfoTrait>(
    info: &AI,
    sol_destination: &AI,
) -> Result<(), AccountError> {
    let system_program_id = [0u8; 32];

    if info.key() == sol_destination.key() {
        info.assign(&system_program_id)?;
        info.realloc(0, false)?;
        return Ok(());
    }

    let lamports_to_transfer = info.lamports();

    sol_destination.add_lamports(lamports_to_transfer)?;
    info.sub_lamports(lamports_to_transfer)?;

    info.assign(&system_program_id)?;
    info.realloc(0, false)?;

    Ok(())
}

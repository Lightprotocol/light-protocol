use solana_account_info::AccountInfo;

use crate::error::{LightSdkError, Result};

// close native solana account
pub fn close<'info>(
    info: &mut AccountInfo<'info>,
    sol_destination: AccountInfo<'info>,
) -> Result<()> {
    let lamports_to_transfer = info.lamports();

    **info
        .try_borrow_mut_lamports()
        .map_err(|_| LightSdkError::ConstraintViolation)? = 0;

    let dest_lamports = sol_destination.lamports();
    **sol_destination
        .try_borrow_mut_lamports()
        .map_err(|_| LightSdkError::ConstraintViolation)? =
        dest_lamports.checked_add(lamports_to_transfer).unwrap();

    let system_program_id = solana_pubkey::pubkey!("11111111111111111111111111111111");

    info.assign(&system_program_id);

    info.resize(0)?;

    Ok(())
}

use solana_account_info::AccountInfo;

use crate::error::{LightSdkError, Result};

// close native solana account
pub fn close<'info>(
    info: &mut AccountInfo<'info>,
    sol_destination: AccountInfo<'info>,
) -> Result<()> {
    let system_program_id = solana_pubkey::pubkey!("11111111111111111111111111111111");

    if info.key == sol_destination.key {
        info.assign(&system_program_id);
        info.realloc(0, false)
            .map_err(|_| LightSdkError::ConstraintViolation)?;
        return Ok(());
    }

    let lamports_to_transfer = info.lamports();

    let new_destination_lamports = sol_destination
        .lamports()
        .checked_add(lamports_to_transfer)
        .ok_or(LightSdkError::ConstraintViolation)?;

    {
        let mut destination_lamports = sol_destination
            .try_borrow_mut_lamports()
            .map_err(|_| LightSdkError::ConstraintViolation)?;
        **destination_lamports = new_destination_lamports;
    }

    {
        let mut source_lamports = info
            .try_borrow_mut_lamports()
            .map_err(|_| LightSdkError::ConstraintViolation)?;
        **source_lamports = 0;
    }

    info.assign(&system_program_id);
    info.realloc(0, false)
        .map_err(|_| LightSdkError::ConstraintViolation)?;

    Ok(())
}

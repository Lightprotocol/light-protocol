use solana_account_info::AccountInfo;

use crate::error::{LightSdkError, Result};

// close native solana account
pub fn close<'info>(
    info: &mut AccountInfo<'info>,
    sol_destination: AccountInfo<'info>,
) -> Result<()> {
    let lamports_to_transfer = info.lamports();

    let new_destination_lamports = sol_destination
        .lamports()
        .checked_add(lamports_to_transfer)
        .ok_or(LightSdkError::ConstraintViolation)?;

    if info.lamports() != lamports_to_transfer {
        return Err(LightSdkError::ConstraintViolation);
    }

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

    let system_program_id = solana_pubkey::pubkey!("11111111111111111111111111111111");
    info.assign(&system_program_id);
    info.resize(0)?;

    Ok(())
}

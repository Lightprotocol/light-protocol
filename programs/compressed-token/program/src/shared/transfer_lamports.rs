use light_profiler::profile;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};
use pinocchio_system::instructions::Transfer;
use spl_pod::solana_msg::msg;

#[profile]
pub fn transfer_lamports(
    amount: u64,
    from: &AccountInfo,
    to: &AccountInfo,
) -> Result<(), ProgramError> {
    let from_lamports: u64 = *from.try_borrow_lamports()?;
    let to_lamports: u64 = *to.try_borrow_lamports()?;
    if from_lamports < amount {
        msg!("payer lamports {}", from_lamports);
        msg!("required lamports {}", amount);
        return Err(ProgramError::InsufficientFunds);
    }

    let from_lamports = from_lamports
        .checked_sub(amount)
        .ok_or(ProgramError::InsufficientFunds)?;
    let to_lamports = to_lamports
        .checked_add(amount)
        .ok_or(ProgramError::InsufficientFunds)?;
    *from.try_borrow_mut_lamports()? = from_lamports;
    *to.try_borrow_mut_lamports()? = to_lamports;
    Ok(())
}

/// Transfer lamports using CPI to system program
/// This is needed when transferring from accounts not owned by our program
#[profile]
pub fn transfer_lamports_via_cpi(
    amount: u64,
    from: &AccountInfo,
    to: &AccountInfo,
) -> Result<(), ProgramError> {
    // Use pinocchio_system's Transfer directly - no type conversions needed
    let transfer = Transfer {
        from,
        to,
        lamports: amount,
    };

    transfer.invoke()
}

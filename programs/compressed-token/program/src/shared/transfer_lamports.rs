use light_program_profiler::profile;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError};
use pinocchio_system::instructions::Transfer as SystemTransfer;
use spl_pod::solana_msg::msg;

/// A transfer instruction containing the recipient account and amount
#[derive(Debug)]
pub struct Transfer<'a> {
    pub account: &'a AccountInfo,
    pub amount: u64,
}

#[inline(always)]
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
#[inline(always)]
#[profile]
pub fn transfer_lamports_via_cpi(
    amount: u64,
    from: &AccountInfo,
    to: &AccountInfo,
) -> Result<(), ProgramError> {
    let transfer = SystemTransfer {
        from,
        to,
        lamports: amount,
    };

    transfer.invoke()
}

/// Multi-transfer optimization that performs a single CPI and manual transfers (pinocchio version)
///
/// Transfers the total amount to the first recipient via CPI, then manually
/// transfers from the first recipient to subsequent recipients. This reduces
/// the number of CPIs from N to 1.
#[inline(always)]
#[profile]
pub fn multi_transfer_lamports(
    payer: &AccountInfo,
    transfers: &[Transfer],
) -> Result<(), ProgramError> {
    // Calculate total amount needed
    let total_amount: u64 = transfers
        .iter()
        .map(|t| t.amount)
        .try_fold(0u64, |acc, amt| acc.checked_add(amt))
        .ok_or(ProgramError::ArithmeticOverflow)?;

    if total_amount == 0 {
        return Ok(());
    }

    // Single CPI to transfer total amount to first recipient
    let first_recipient = transfers[0].account;
    transfer_lamports_via_cpi(total_amount, payer, first_recipient)?;

    // Manual transfers from first recipient to subsequent recipients
    for transfer in transfers.iter().skip(1) {
        if transfer.amount > 0 {
            transfer_lamports(transfer.amount, first_recipient, transfer.account)?;
        }
    }

    Ok(())
}

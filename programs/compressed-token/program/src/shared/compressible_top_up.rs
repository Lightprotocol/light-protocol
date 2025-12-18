use anchor_lang::solana_program::program_error::ProgramError;
use light_ctoken_interface::{
    state::{CToken, CompressedMint, ZExtensionStruct},
    CTokenError, BASE_TOKEN_ACCOUNT_SIZE,
};
use light_program_profiler::profile;
use light_zero_copy::traits::ZeroCopyAt;
use pinocchio::{
    account_info::AccountInfo,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
};

use super::{
    convert_program_error,
    transfer_lamports::{multi_transfer_lamports, Transfer},
};

/// Calculate and execute top-up transfers for compressible CMint and CToken accounts.
/// Both accounts are optional - if an account doesn't have compressible extension, it's skipped.
///
/// # Arguments
/// * `cmint` - The CMint account (may or may not have Compressible extension)
/// * `ctoken` - The CToken account (may or may not have Compressible extension)
/// * `payer` - The fee payer for top-ups
/// * `max_top_up` - Maximum lamports for top-ups combined (0 = no limit)
#[inline(always)]
#[profile]
pub fn calculate_and_execute_compressible_top_ups<'a>(
    cmint: &'a AccountInfo,
    ctoken: &'a AccountInfo,
    payer: &'a AccountInfo,
    max_top_up: u16,
) -> Result<(), ProgramError> {
    let mut transfers = [
        Transfer {
            account: cmint,
            amount: 0,
        },
        Transfer {
            account: ctoken,
            amount: 0,
        },
    ];

    let mut current_slot = 0;
    let mut rent: Option<Rent> = None;
    // Initialize budget: +1 allows exact match (total == max_top_up)
    let mut lamports_budget = (max_top_up as u64).saturating_add(1);

    // Calculate CMint top-up using zero-copy
    {
        let cmint_data = cmint.try_borrow_data().map_err(convert_program_error)?;
        let (mint, _) = CompressedMint::zero_copy_at(&cmint_data)
            .map_err(|_| CTokenError::CMintDeserializationFailed)?;
        if let Some(ref extensions) = mint.extensions {
            for extension in extensions.iter() {
                if let ZExtensionStruct::Compressible(ref compression_info) = extension {
                    if current_slot == 0 {
                        current_slot = Clock::get()
                            .map_err(|_| CTokenError::SysvarAccessError)?
                            .slot;
                        rent = Some(Rent::get().map_err(|_| CTokenError::SysvarAccessError)?);
                    }
                    let rent_exemption = rent.as_ref().unwrap().minimum_balance(cmint.data_len());
                    transfers[0].amount = compression_info
                        .info
                        .calculate_top_up_lamports(
                            cmint.data_len() as u64,
                            current_slot,
                            cmint.lamports(),
                            rent_exemption,
                        )
                        .map_err(|_| CTokenError::InvalidAccountData)?;
                    lamports_budget = lamports_budget.saturating_sub(transfers[0].amount);
                    break;
                }
            }
        }
    }

    // Calculate CToken top-up (CToken uses zero-copy extensions)
    if ctoken.data_len() > BASE_TOKEN_ACCOUNT_SIZE as usize {
        let account_data = ctoken.try_borrow_data().map_err(convert_program_error)?;
        let (token, _) = CToken::zero_copy_at_checked(&account_data)?;
        if let Some(ref extensions) = token.extensions {
            for extension in extensions.iter() {
                if let ZExtensionStruct::Compressible(compressible_ext) = extension {
                    if current_slot == 0 {
                        current_slot = Clock::get()
                            .map_err(|_| CTokenError::SysvarAccessError)?
                            .slot;
                        rent = Some(Rent::get().map_err(|_| CTokenError::SysvarAccessError)?);
                    }
                    let rent_exemption = rent.as_ref().unwrap().minimum_balance(ctoken.data_len());
                    transfers[1].amount = compressible_ext
                        .info
                        .calculate_top_up_lamports(
                            ctoken.data_len() as u64,
                            current_slot,
                            ctoken.lamports(),
                            rent_exemption,
                        )
                        .map_err(|_| CTokenError::InvalidAccountData)?;
                    lamports_budget = lamports_budget.saturating_sub(transfers[1].amount);
                    break;
                }
            }
        } else {
            // Only Compressible extensions are implemented for ctoken accounts.
            return Err(CTokenError::InvalidAccountData.into());
        }
    }

    // Exit early if no compressible accounts
    if current_slot == 0 {
        return Ok(());
    }

    if transfers[0].amount == 0 && transfers[1].amount == 0 {
        return Ok(());
    }

    // Check budget wasn't exhausted (0 means exceeded max_top_up)
    if max_top_up != 0 && lamports_budget == 0 {
        return Err(CTokenError::MaxTopUpExceeded.into());
    }

    multi_transfer_lamports(payer, &transfers).map_err(convert_program_error)?;
    Ok(())
}

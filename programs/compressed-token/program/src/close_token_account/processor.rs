use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::{checks::check_signer, AccountInfoTrait};
use light_compressible::rent::{get_rent_exemption_lamports, AccountRentState};
use light_ctoken_types::state::{CToken, ZCompressedTokenMut, ZExtensionStructMut};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;
#[cfg(target_os = "solana")]
use pinocchio::sysvars::Sysvar;
use spl_pod::solana_msg::msg;
use spl_token_2022::state::AccountState;

use super::accounts::CloseTokenAccountAccounts;
use crate::shared::{convert_program_error, transfer_lamports};

/// Process the close token account instruction
#[profile]
pub fn process_close_token_account(
    account_infos: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Validate and get accounts
    let accounts = CloseTokenAccountAccounts::validate_and_parse(account_infos)?;
    {
        // Try to parse as CToken using zero-copy deserialization
        let token_account_data =
            &mut AccountInfoTrait::try_borrow_mut_data(accounts.token_account)?;
        let (ctoken, _) = CToken::zero_copy_at_mut_checked(token_account_data)?;
        validate_token_account_close_instruction(&accounts, &ctoken)?;
    }
    close_token_account(&accounts)?;
    Ok(())
}

/// Validates that a ctoken solana account is ready to be closed.
/// The rent authority cannot close the account.
#[profile]
pub fn validate_token_account_close_instruction(
    accounts: &CloseTokenAccountAccounts,
    ctoken: &ZCompressedTokenMut<'_>,
) -> Result<(bool, bool), ProgramError> {
    validate_token_account::<false>(accounts, ctoken)
}

/// Validates that a ctoken solana account is ready to be closed.
/// The rent authority can close the account.
#[profile]
pub fn validate_token_account_for_close_transfer2(
    accounts: &CloseTokenAccountAccounts,
    ctoken: &ZCompressedTokenMut<'_>,
) -> Result<(bool, bool), ProgramError> {
    validate_token_account::<true>(accounts, ctoken)
}

#[inline(always)]
fn validate_token_account<const COMPRESS_AND_CLOSE: bool>(
    accounts: &CloseTokenAccountAccounts,
    ctoken: &ZCompressedTokenMut<'_>,
) -> Result<(bool, bool), ProgramError> {
    if accounts.token_account.key() == accounts.destination.key() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Check account state - reject frozen and uninitialized
    match *ctoken.state {
        state if state == AccountState::Initialized as u8 => {} // OK to proceed
        state if state == AccountState::Frozen as u8 => return Err(ErrorCode::AccountFrozen.into()),
        _ => return Err(ProgramError::UninitializedAccount),
    }
    // For compress and close we compress the balance and close.
    if !COMPRESS_AND_CLOSE {
        // Check that the account has zero balance
        if u64::from(*ctoken.amount) != 0 {
            return Err(ErrorCode::NonNativeHasBalance.into());
        }
    }
    // Verify the authority matches the account owner or rent authority (if compressible)
    let owner_matches = ctoken.owner.to_bytes() == *accounts.authority.key();
    if let Some(extensions) = ctoken.extensions.as_ref() {
        // Look for compressible extension
        for extension in extensions {
            if let ZExtensionStructMut::Compressible(compressible_ext) = extension {
                let rent_sponsor = accounts
                    .rent_sponsor
                    .ok_or(ProgramError::NotEnoughAccountKeys)?;
                if compressible_ext.rent_sponsor != *rent_sponsor.key() {
                    msg!("rent recipient mismatch");
                    return Err(ProgramError::InvalidAccountData);
                }

                if COMPRESS_AND_CLOSE {
                    #[allow(clippy::collapsible_if)]
                    if !owner_matches {
                        if compressible_ext.compression_authority != *accounts.authority.key() {
                            msg!("rent authority mismatch");
                            return Err(ProgramError::InvalidAccountData);
                        }

                        #[cfg(target_os = "solana")]
                        use pinocchio::sysvars::Sysvar;
                        #[cfg(target_os = "solana")]
                        let current_slot = pinocchio::sysvars::clock::Clock::get()
                            .map_err(convert_program_error)?
                            .slot;

                        // For rent authority, check timing constraints
                        #[cfg(target_os = "solana")]
                        {
                            let is_compressible = compressible_ext
                                .is_compressible(
                                    accounts.token_account.data_len() as u64,
                                    current_slot,
                                    accounts.token_account.lamports(),
                                )
                                .map_err(|_| ProgramError::InvalidAccountData)?;

                            if is_compressible.is_none() {
                                msg!("account not compressible");
                                return Err(ProgramError::InvalidAccountData);
                            } else {
                                return Ok((true, compressible_ext.compress_to_pubkey()));
                            }
                        }
                    }
                }
                // Check if authority is the rent authority && rent_sponsor is the destination account
            }
        }
    }
    if !owner_matches {
        msg!(
            "owner: ctoken.owner {:?} != {:?} authority",
            solana_pubkey::Pubkey::from(ctoken.owner.to_bytes()),
            solana_pubkey::Pubkey::from(*accounts.authority.key())
        );
        // If we have no rent authority owner must match
        return Err(ErrorCode::OwnerMismatch.into());
    }
    Ok((false, false))
}

pub fn close_token_account(accounts: &CloseTokenAccountAccounts<'_>) -> Result<(), ProgramError> {
    distribute_lamports(accounts)?;
    finalize_account_closure(accounts)
}

#[profile]
pub fn distribute_lamports(accounts: &CloseTokenAccountAccounts<'_>) -> Result<(), ProgramError> {
    let token_account_lamports = AccountInfoTrait::lamports(accounts.token_account);
    // Additional signer check is necessary for usage in transfer2.
    check_signer(accounts.authority).map_err(|e| {
        anchor_lang::solana_program::msg!("Authority signer check failed: {:?}", e);
        ProgramError::from(e)
    })?;
    // Check for compressible extension and handle lamport distribution

    let token_account_data = AccountInfoTrait::try_borrow_data(accounts.token_account)?;
    let (ctoken, _) = CToken::zero_copy_at_checked(&token_account_data)?;

    if let Some(extensions) = ctoken.extensions.as_ref() {
        for extension in extensions {
            if let light_ctoken_types::state::ZExtensionStruct::Compressible(compressible_ext) =
                extension
            {
                // Calculate distribution based on rent and write_top_up
                #[cfg(target_os = "solana")]
                let current_slot = pinocchio::sysvars::clock::Clock::get()
                    .map_err(convert_program_error)?
                    .slot;
                #[cfg(not(target_os = "solana"))]
                let current_slot = 0;
                let compression_cost: u64 = compressible_ext.rent_config.compression_cost.into();

                let (mut lamports_to_rent_sponsor, mut lamports_to_destination) = {
                    let base_lamports =
                        get_rent_exemption_lamports(accounts.token_account.data_len() as u64)
                            .map_err(|_| ProgramError::InvalidAccountData)?;

                    let state = AccountRentState {
                        num_bytes: accounts.token_account.data_len() as u64,
                        current_slot,
                        current_lamports: token_account_lamports,
                        last_claimed_slot: compressible_ext.last_claimed_slot.into(),
                    };

                    let distribution = state
                        .calculate_close_distribution(&compressible_ext.rent_config, base_lamports);
                    (distribution.to_rent_sponsor, distribution.to_user)
                };

                let rent_sponsor = accounts
                    .rent_sponsor
                    .ok_or(ProgramError::NotEnoughAccountKeys)?;

                if accounts.authority.key() == &compressible_ext.compression_authority {
                    // When compressing via compression_authority:
                    // Extract compression incentive from rent_sponsor portion to give to forester
                    // The compression incentive is included in lamports_to_rent_sponsor
                    lamports_to_rent_sponsor = lamports_to_rent_sponsor
                        .checked_sub(compression_cost)
                        .ok_or(ProgramError::InsufficientFunds)?;

                    // Unused funds also go to rent_sponsor.
                    lamports_to_rent_sponsor += lamports_to_destination;
                    lamports_to_destination = compression_cost; // This will go to fee_payer (forester)
                }

                // Transfer lamports to rent sponsor.
                if lamports_to_rent_sponsor > 0 {
                    transfer_lamports(
                        lamports_to_rent_sponsor,
                        accounts.token_account,
                        rent_sponsor,
                    )
                    .map_err(convert_program_error)?;
                }

                // Transfer lamports to destination (user or forester).
                if lamports_to_destination > 0 {
                    transfer_lamports(
                        lamports_to_destination,
                        accounts.token_account,
                        accounts.destination,
                    )
                    .map_err(convert_program_error)?;
                }
                return Ok(());
            }
        }
    }

    // Non-compressible account: transfer all lamports to destination
    if token_account_lamports > 0 {
        transfer_lamports(
            token_account_lamports,
            accounts.token_account,
            accounts.destination,
        )
        .map_err(convert_program_error)?;
    }
    Ok(())
}

fn finalize_account_closure(accounts: &CloseTokenAccountAccounts<'_>) -> Result<(), ProgramError> {
    unsafe {
        accounts.token_account.assign(&[0u8; 32]);
    }
    match accounts.token_account.resize(0) {
        Ok(()) => Ok(()),
        Err(e) => Err(ProgramError::Custom(u64::from(e) as u32 + 6000)),
    }
}

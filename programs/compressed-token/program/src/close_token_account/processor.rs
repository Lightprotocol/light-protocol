use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::{checks::check_signer, AccountInfoTrait};
use light_compressible::rent::{calculate_close_lamports, get_rent_exemption_lamports};
use light_ctoken_types::state::{CompressedToken, ZCompressedTokenMut, ZExtensionStructMut};
use light_profiler::profile;
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use pinocchio::account_info::AccountInfo;
#[cfg(target_os = "solana")]
use pinocchio::sysvars::Sysvar;
use spl_pod::solana_msg::msg;
use spl_token_2022::state::AccountState;

use super::accounts::CloseTokenAccountAccounts;
use crate::shared::transfer_lamports;

/// Process the close token account instruction
#[profile]
pub fn process_close_token_account(
    account_infos: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Validate and get accounts
    let accounts = CloseTokenAccountAccounts::validate_and_parse(account_infos)?;
    {
        // Try to parse as CompressedToken using zero-copy deserialization
        let token_account_data =
            &mut AccountInfoTrait::try_borrow_mut_data(accounts.token_account)?;
        let (compressed_token, _) = CompressedToken::zero_copy_at_mut(token_account_data)?;
        // validate_and_close_token_account(&accounts, &compressed_token)?;
        // The rent authority cannot close the account.
        validate_token_account::<false>(&accounts, &compressed_token)?;
    }
    close_token_account(&accounts)?;
    Ok(())
}

#[profile]
pub fn validate_token_account<const CHECK_RENT_AUTH: bool>(
    accounts: &CloseTokenAccountAccounts,
    compressed_token: &ZCompressedTokenMut<'_>,
) -> Result<(bool, bool), ProgramError> {
    if accounts.token_account.key() == accounts.destination.key() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Check account state - reject frozen and uninitialized
    match *compressed_token.state {
        state if state == AccountState::Initialized as u8 => {} // OK to proceed
        state if state == AccountState::Frozen as u8 => return Err(ErrorCode::AccountFrozen.into()),
        _ => return Err(ProgramError::UninitializedAccount),
    }
    if !CHECK_RENT_AUTH {
        // Check that the account has zero balance
        if u64::from(*compressed_token.amount) != 0 {
            return Err(ErrorCode::NonNativeHasBalance.into());
        }
    }
    // Verify the authority matches the account owner or rent authority (if compressible)
    let owner_matches = compressed_token.owner.to_bytes() == *accounts.authority.key();
    if let Some(extensions) = compressed_token.extensions.as_ref() {
        // Look for compressible extension
        for extension in extensions {
            if let ZExtensionStructMut::Compressible(compressible_ext) = extension {
                if compressible_ext.rent_recipient != *accounts.destination.key() {
                    msg!("rent recipient missmatch");
                    return Err(ProgramError::InvalidAccountData);
                }

                if CHECK_RENT_AUTH {
                    #[allow(clippy::collapsible_if)]
                    if !owner_matches {
                        if compressible_ext.rent_authority != *accounts.authority.key() {
                            msg!("rent authority missmatch");
                            return Err(ProgramError::InvalidAccountData);
                        }
                        #[cfg(target_os = "solana")]
                        use pinocchio::sysvars::Sysvar;
                        #[cfg(target_os = "solana")]
                        let current_slot = pinocchio::sysvars::clock::Clock::get()
                            .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?
                            .slot;

                        // For rent authority, check timing constraints
                        #[cfg(target_os = "solana")]
                        {
                            let (is_compressible, _) = compressible_ext
                                .is_compressible(
                                    accounts.token_account.data_len() as u64,
                                    current_slot,
                                    accounts.token_account.lamports(),
                                )
                                .map_err(|_| ProgramError::InvalidAccountData)?;

                            if !is_compressible {
                                msg!("account not compressible");
                                return Err(ProgramError::InvalidAccountData);
                            } else {
                                return Ok((true, compressible_ext.compress_to_pubkey()));
                            }
                        }
                    }
                }
                // Check if authority is the rent authority && rent_recipient is the destination account
            }
        }
    }
    if !owner_matches {
        msg!(
            "owner: compressed_token.owner {:?} != {:?} authority",
            solana_pubkey::Pubkey::from(compressed_token.owner.to_bytes()),
            solana_pubkey::Pubkey::from(*accounts.authority.key())
        );
        // If we have no rent authority owner must match
        return Err(ErrorCode::OwnerMismatch.into());
    }
    Ok((false, false))
}

pub fn close_token_account(accounts: &CloseTokenAccountAccounts<'_>) -> Result<(), ProgramError> {
    close_token_account_inner(accounts)?;
    finalize_account_closure(accounts)
}
pub fn close_token_account_inner(
    accounts: &CloseTokenAccountAccounts<'_>,
) -> Result<(), ProgramError> {
    let token_account_lamports = AccountInfoTrait::lamports(accounts.token_account);
    check_signer(accounts.authority).map_err(|e| {
        anchor_lang::solana_program::msg!("Authority signer check failed: {:?}", e);
        ProgramError::from(e)
    })?;
    // Check for compressible extension and handle lamport distribution
    {
        let token_account_data = AccountInfoTrait::try_borrow_data(accounts.token_account)?;
        let (compressed_token, _) = CompressedToken::zero_copy_at(&token_account_data)?;

        if let Some(extensions) = compressed_token.extensions.as_ref() {
            for extension in extensions {
                if let light_ctoken_types::state::ZExtensionStruct::Compressible(compressible_ext) =
                    extension
                {
                    // Calculate distribution based on rent and write_top_up
                    #[cfg(target_os = "solana")]
                    let current_slot = pinocchio::sysvars::clock::Clock::get()
                        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?
                        .slot;
                    #[cfg(not(target_os = "solana"))]
                    let current_slot = 0;

                    let base_lamports =
                        get_rent_exemption_lamports(accounts.token_account.data_len() as u64)
                            .map_err(|_| ProgramError::InvalidAccountData)?;

                    let min_rent: u64 = compressible_ext.rent_config.min_rent.into();
                    let rent_per_byte: u64 = compressible_ext.rent_config.rent_per_byte.into();
                    let full_compression_incentive: u64 = compressible_ext
                        .rent_config
                        .full_compression_incentive
                        .into();

                    let (mut lamports_to_destination, mut lamports_to_authority) =
                        calculate_close_lamports(
                            accounts.token_account.data_len() as u64,
                            current_slot,
                            token_account_lamports,
                            compressible_ext.last_claimed_slot,
                            base_lamports,
                            min_rent,
                            rent_per_byte,
                            full_compression_incentive,
                        );

                    if accounts.authority.key() == &compressible_ext.rent_authority {
                        lamports_to_destination += lamports_to_authority;
                        lamports_to_authority = 0;
                    }

                    // Transfer lamports to destination (rent recipient)
                    if lamports_to_destination > 0 {
                        transfer_lamports(
                            lamports_to_destination,
                            accounts.token_account,
                            accounts.destination,
                        )
                        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;
                    }

                    // Transfer lamports to authority (fee payer) if any write_top_up
                    if lamports_to_authority > 0 {
                        transfer_lamports(
                            lamports_to_authority,
                            accounts.token_account,
                            accounts.authority,
                        )
                        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;
                    }
                    return Ok(());
                }
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
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;
    }
    Ok(())
}

fn finalize_account_closure(accounts: &CloseTokenAccountAccounts<'_>) -> Result<(), ProgramError> {
    unsafe {
        accounts.token_account.assign(&[0u8; 32]);
    }
    // Prevent account revival attack by reallocating to 0 bytes
    match accounts.token_account.resize(0) {
        Ok(()) => Ok(()),
        Err(e) => Err(ProgramError::Custom(u64::from(e) as u32)),
    }
}

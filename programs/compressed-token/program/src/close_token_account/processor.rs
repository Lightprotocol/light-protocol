use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountInfoTrait;
use light_ctoken_types::state::{CompressedToken, ZExtensionStruct};
use light_zero_copy::borsh::Deserialize;
use pinocchio::account_info::AccountInfo;
use spl_token_2022::state::AccountState;

use super::accounts::CloseTokenAccountAccounts;

/// Process the close token account instruction
pub fn process_close_token_account(
    account_infos: &[AccountInfo],
    _instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Validate and get accounts
    let accounts = CloseTokenAccountAccounts::validate_and_parse(account_infos)?;

    // Validate token account state and balance
    {
        let token_account_data = AccountInfoTrait::try_borrow_data(accounts.token_account)
            .map_err(|_| ProgramError::InvalidAccountData)?;

        // Try to parse as CompressedToken using zero-copy deserialization
        let (compressed_token, _) = CompressedToken::zero_copy_at(&token_account_data)
            .map_err(|_| ProgramError::InvalidAccountData)?;

        // Check that the account is initialized
        if compressed_token.state != AccountState::Initialized as u8 {
            return Err(ProgramError::UninitializedAccount);
        }

        // Check that the account has zero balance
        if u64::from(*compressed_token.amount) != 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        // Verify the authority matches the account owner or rent authority (if compressible)
        let authority_key = solana_pubkey::Pubkey::new_from_array(*accounts.authority.key());
        let mut is_valid_authority = compressed_token.owner.to_bytes() == authority_key.to_bytes();

        // Check if account has compressible extension and if authority is rent authority
        if !is_valid_authority {
            if let Some(extensions) = compressed_token.extensions.as_ref() {
                // Look for compressible extension
                for extension in extensions {
                    if let ZExtensionStruct::Compressible(compressible_ext) = extension {
                        // Check if authority is the rent authority
                        if compressible_ext.rent_authority.to_bytes() == authority_key.to_bytes() {
                            is_valid_authority = true;

                            // For rent authority, check timing constraints
                            #[cfg(target_os = "solana")]
                            if !compressible_ext.is_compressible()? {
                                return Err(ProgramError::InvalidAccountData);
                            }
                            break;
                        }
                    }
                }
            }
        }

        if !is_valid_authority {
            return Err(ProgramError::InvalidAccountOwner);
        }
    }
    // TODO: double check that it is safely closed.
    // Transfer all lamports from token account to destination
    let token_account_lamports = AccountInfoTrait::lamports(accounts.token_account);

    // Set token account lamports to 0
    unsafe {
        *accounts.token_account.borrow_mut_lamports_unchecked() = 0;
    }

    // Add lamports to destination
    let destination_lamports = AccountInfoTrait::lamports(accounts.destination);
    let new_destination_lamports = destination_lamports
        .checked_add(token_account_lamports)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    unsafe {
        *accounts.destination.borrow_mut_lamports_unchecked() = new_destination_lamports;
    }
    // Clear the token account data
    let mut token_account_data = AccountInfoTrait::try_borrow_mut_data(accounts.token_account)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    token_account_data.fill(0);

    Ok(())
}

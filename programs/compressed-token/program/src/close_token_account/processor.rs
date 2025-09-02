use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountInfoTrait;
use light_ctoken_types::state::{CompressedToken, ZCompressedTokenMut, ZExtensionStructMut};
use light_zero_copy::traits::ZeroCopyAtMut;
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
    {
        // Try to parse as CompressedToken using zero-copy deserialization
        let token_account_data =
            &mut AccountInfoTrait::try_borrow_mut_data(accounts.token_account)?;
        let (compressed_token, _) = CompressedToken::zero_copy_at_mut(token_account_data)?;
        // validate_and_close_token_account(&accounts, &compressed_token)?;
        validate_token_account(&accounts, &compressed_token)?;
    }
    close_token_account(&accounts)?;
    Ok(())
}

pub fn validate_token_account(
    accounts: &CloseTokenAccountAccounts,
    compressed_token: &ZCompressedTokenMut<'_>,
) -> Result<(), ProgramError> {
    if accounts.token_account.key() == accounts.destination.key() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Check account state - reject frozen and uninitialized
    match *compressed_token.state {
        state if state == AccountState::Initialized as u8 => {} // OK to proceed
        state if state == AccountState::Frozen as u8 => return Err(ErrorCode::AccountFrozen.into()),
        _ => return Err(ProgramError::UninitializedAccount),
    }

    // Check that the account has zero balance
    if u64::from(*compressed_token.amount) != 0 {
        return Err(ErrorCode::NonNativeHasBalance.into());
    }

    // Verify the authority matches the account owner or rent authority (if compressible)
    if compressed_token.owner.to_bytes() == *accounts.authority.key() {
        return Ok(());
    } else if let Some(extensions) = compressed_token.extensions.as_ref() {
        // Look for compressible extension
        for extension in extensions {
            if let ZExtensionStructMut::Compressible(compressible_ext) = extension {
                // Check if authority is the rent authority && rent_recipient is the destination account
                if compressible_ext.rent_authority.to_bytes() == *accounts.authority.key()
                    && compressible_ext.rent_recipient.to_bytes() == *accounts.destination.key()
                {
                    // For rent authority, check timing constraints
                    #[cfg(target_os = "solana")]
                    if !compressible_ext.is_compressible()? {
                        return Err(ProgramError::InvalidAccountData);
                    } else {
                        return Ok(());
                    }
                }
            }
        }
    }

    Err(ErrorCode::OwnerMismatch.into())
}

pub fn close_token_account(accounts: &CloseTokenAccountAccounts<'_>) -> Result<(), ProgramError> {
    let token_account_lamports = AccountInfoTrait::lamports(accounts.token_account);

    // SAFETY: Required for direct lamport manipulation, account validated above
    unsafe {
        *accounts.token_account.borrow_mut_lamports_unchecked() = 0;
    }

    let destination_lamports = AccountInfoTrait::lamports(accounts.destination);
    let new_destination_lamports = destination_lamports
        .checked_add(token_account_lamports)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // SAFETY: Required for direct lamport manipulation, overflow checked above
    unsafe {
        *accounts.destination.borrow_mut_lamports_unchecked() = new_destination_lamports;
    }

    unsafe {
        accounts.token_account.assign(&[0u8; 32]);
    }
    // Prevent account revival attack by reallocating to 0 bytes
    match accounts.token_account.realloc(0, false) {
        Ok(()) => {}
        Err(e) => return Err(ProgramError::Custom(u64::from(e) as u32)),
    }

    Ok(())
}

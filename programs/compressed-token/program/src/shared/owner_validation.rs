use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::check_signer;
use light_ctoken_types::state::ZCompressedTokenMut;
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;

/// Verify owner or delegate signer authorization for token operations
/// Returns the delegate account info if delegate is used, None otherwise
#[profile]
pub fn verify_owner_or_delegate_signer<'a>(
    owner_account: &'a AccountInfo,
    delegate_account: Option<&'a AccountInfo>,
) -> Result<(), ProgramError> {
    if let Some(delegate_account) = delegate_account {
        // If delegate is used, delegate or owner must be signer
        match check_signer(delegate_account) {
            Ok(()) => {}
            Err(delegate_error) => {
                check_signer(owner_account).map_err(|e| {
                    anchor_lang::solana_program::msg!(
                        "Checking owner signer: {:?}",
                        solana_pubkey::Pubkey::new_from_array(*owner_account.key())
                    );
                    anchor_lang::solana_program::msg!("Owner signer check failed: {:?}", e);
                    anchor_lang::solana_program::msg!(
                        "Delegate signer: {:?}",
                        solana_pubkey::Pubkey::new_from_array(*delegate_account.key())
                    );
                    anchor_lang::solana_program::msg!(
                        "Delegate signer check failed: {:?}",
                        delegate_error
                    );
                    ProgramError::from(e)
                })?;
            }
        }
        Ok(())
    } else {
        // If no delegate, owner must be signer
        check_signer(owner_account).map_err(|e| {
            anchor_lang::solana_program::msg!(
                "Checking owner signer: {:?}",
                solana_pubkey::Pubkey::new_from_array(*owner_account.key())
            );
            anchor_lang::solana_program::msg!("Owner signer check failed: {:?}", e);
            ProgramError::from(e)
        })?;
        Ok(())
    }
}

/// Verify and update token account authority using zero-copy compressed token format
#[profile]
pub fn check_ctoken_owner(
    compressed_token: &mut ZCompressedTokenMut,
    authority_account: &AccountInfo,
) -> Result<(), ProgramError> {
    // Verify authority is signer
    check_signer(authority_account).map_err(|e| {
        anchor_lang::solana_program::msg!("Authority signer check failed: {:?}", e);
        ProgramError::from(e)
    })?;

    let authority_key = authority_account.key();
    let owner_key = compressed_token.owner.to_bytes();

    // Check if authority is the owner
    if *authority_key == owner_key {
        Ok(()) // Owner can always compress, no delegation update needed
    } else {
        Err(ErrorCode::OwnerMismatch.into())
    }
    // delegation is unimplemented.
    // // Check if authority is a valid delegate
    // if let Some(delegate) = &compressed_token.delegate {
    //     let delegate_key = delegate.to_bytes();
    //     if *authority_key == delegate_key {
    //         // Verify delegated amount is sufficient
    //         let delegated_amount: u64 = u64::from(*compressed_token.delegated_amount);
    //         if delegated_amount >= compression_amount {
    //             // Decrease delegated amount by compression amount
    //             let new_delegated_amount = delegated_amount
    //                 .checked_sub(compression_amount)
    //                 .ok_or(ProgramError::ArithmeticOverflow)?;
    //             *compressed_token.delegated_amount = new_delegated_amount.into();
    //             return Ok(());
    //         } else {
    //             anchor_lang::solana_program::msg!(
    //                 "Insufficient delegated amount: {} < {}",
    //                 delegated_amount,
    //                 compression_amount
    //             );
    //             return Err(ProgramError::InsufficientFunds);
    //         }
    //     }
    // }
    // Authority is neither owner, valid delegate, nor rent authority
}

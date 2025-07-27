use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::check_signer;
use pinocchio::account_info::AccountInfo;
use spl_token_2022::pod::PodAccount;

/// Verify owner or delegate signer authorization for token operations
/// Returns the delegate account info if delegate is used, None otherwise
pub fn verify_owner_or_delegate_signer<'a>(
    owner_account: &'a AccountInfo,
    delegate_account: Option<&'a AccountInfo>,
) -> Result<Option<&'a AccountInfo>, ProgramError> {
    if let Some(delegate_account) = delegate_account {
        // If delegate is used, delegate must be signer
        check_signer(delegate_account).map_err(|e| {
            anchor_lang::solana_program::msg!(
                "Delegate signer: {:?}",
                solana_pubkey::Pubkey::new_from_array(*delegate_account.key())
            );
            anchor_lang::solana_program::msg!("Delegate signer check failed: {:?}", e);
            ProgramError::from(e)
        })?;
        Ok(Some(delegate_account))
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
        Ok(None)
    }
}

/// Verify authority for token account compression operations using existing pod_account
/// Checks if authority is owner or valid delegate with sufficient delegated amount
/// If delegate, decreases the delegated amount by the compression amount
pub fn verify_and_update_token_account_authority_with_pod(
    pod_account: &mut PodAccount,
    authority_account: &AccountInfo,
    compression_amount: u64,
) -> Result<(), ProgramError> {
    // Verify authority is signer
    check_signer(authority_account).map_err(|e| {
        anchor_lang::solana_program::msg!(
            "Authority signer check failed: {:?}", e
        );
        ProgramError::from(e)
    })?;

    let authority_key = authority_account.key();
    let owner_key = &pod_account.owner;
    
    // Check if authority is the owner  
    if *authority_key == owner_key.to_bytes() {
        return Ok(()); // Owner can always compress, no delegation update needed
    }
    
    // Check if authority is a valid delegate
    if pod_account.delegate.is_some() {
        let delegate_key = pod_account.delegate.ok_or(ProgramError::InvalidAccountData)?;
        if *authority_key == delegate_key.to_bytes() {
            // Verify delegated amount is sufficient
            let delegated_amount: u64 = pod_account.delegated_amount.into();
            if delegated_amount >= compression_amount {
                // Decrease delegated amount by compression amount
                let new_delegated_amount = delegated_amount - compression_amount;
                pod_account.delegated_amount = new_delegated_amount.into();
                
                anchor_lang::solana_program::msg!(
                    "Delegate compression: decreased delegated amount from {} to {}", 
                    delegated_amount, new_delegated_amount
                );
                return Ok(());
            } else {
                anchor_lang::solana_program::msg!(
                    "Insufficient delegated amount: {} < {}", 
                    delegated_amount, compression_amount
                );
                return Err(ProgramError::InsufficientFunds);
            }
        }
    }
    
    // Authority is neither owner nor valid delegate
    anchor_lang::solana_program::msg!(
        "Authority {:?} is not owner or valid delegate of token account", 
        authority_key
    );
    Err(ProgramError::InvalidAccountData)
}
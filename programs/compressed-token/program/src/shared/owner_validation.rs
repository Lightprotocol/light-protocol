use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::check_signer;
use light_ctoken_interface::{state::ZCTokenMut, CTOKEN_PROGRAM_ID};
use light_program_profiler::profile;
use pinocchio::{account_info::AccountInfo, pubkey::pubkey_eq};

use crate::extensions::MintExtensionChecks;

const SPL_TOKEN_ID: [u8; 32] = spl_token::ID.to_bytes();
const SPL_TOKEN_2022_ID: [u8; 32] = spl_token_2022::ID.to_bytes();

/// Check that an account is owned by a valid token program (SPL Token, Token-2022, or cToken).
#[inline(always)]
pub fn check_token_program_owner(account: &AccountInfo) -> Result<(), ProgramError> {
    let owner = account.owner();
    if pubkey_eq(owner, &SPL_TOKEN_ID)
        || pubkey_eq(owner, &SPL_TOKEN_2022_ID)
        || pubkey_eq(owner, &CTOKEN_PROGRAM_ID)
    {
        Ok(())
    } else {
        Err(ProgramError::IncorrectProgramId)
    }
}

/// Verify owner, delegate, or permanent delegate signer authorization for token operations.
/// Accepts optional permanent delegate pubkey from mint extension for additional authorization.
#[profile]
pub fn verify_owner_or_delegate_signer<'a>(
    owner_account: &'a AccountInfo,
    delegate_account: Option<&'a AccountInfo>,
    permanent_delegate: Option<&pinocchio::pubkey::Pubkey>,
    accounts: &[AccountInfo],
) -> Result<(), ProgramError> {
    // Check if owner is signer
    if check_signer(owner_account).is_ok() {
        return Ok(());
    }

    // Check if delegate is signer
    if let Some(delegate_account) = delegate_account {
        if check_signer(delegate_account).is_ok() {
            return Ok(());
        }
    }

    // Check if permanent delegate is signer (search through all accounts)
    if let Some(perm_delegate) = permanent_delegate {
        for account in accounts {
            if account.key() == perm_delegate && account.is_signer() {
                return Ok(());
            }
        }
    }

    // No valid signer found
    anchor_lang::solana_program::msg!(
        "Checking owner signer: {:?}",
        solana_pubkey::Pubkey::new_from_array(*owner_account.key())
    );
    anchor_lang::solana_program::msg!("Owner signer check failed: InvalidSigner");
    if let Some(delegate_account) = delegate_account {
        anchor_lang::solana_program::msg!(
            "Delegate signer: {:?}",
            solana_pubkey::Pubkey::new_from_array(*delegate_account.key())
        );
        anchor_lang::solana_program::msg!("Delegate signer check failed: InvalidSigner");
    }
    if let Some(perm_delegate) = permanent_delegate {
        anchor_lang::solana_program::msg!(
            "Permanent delegate: {:?}",
            solana_pubkey::Pubkey::new_from_array(*perm_delegate)
        );
        anchor_lang::solana_program::msg!("Permanent delegate signer check failed: InvalidSigner");
    }
    Err(ErrorCode::OwnerMismatch.into())
}

/// Verify and update token account authority using zero-copy compressed token format.
/// Allows owner, account delegate, or permanent delegate (from mint) to authorize compression operations.
#[profile]
pub fn check_ctoken_owner(
    compressed_token: &mut ZCTokenMut,
    authority_account: &AccountInfo,
    mint_checks: Option<&MintExtensionChecks>,
    _compression_amount: u64,
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
        return Ok(()); // Owner can always compress
    }

    // Check if authority is the permanent delegate from the mint
    if let Some(checks) = mint_checks {
        if let Some(permanent_delegate) = &checks.permanent_delegate {
            if authority_key == permanent_delegate {
                return Ok(()); // Permanent delegate can compress any account of this mint
            }
        }
    }

    // Authority is neither owner, account delegate, nor permanent delegate
    Err(ErrorCode::OwnerMismatch.into())
}

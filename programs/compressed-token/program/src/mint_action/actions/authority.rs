use std::panic::Location;

use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_compressed_account::Pubkey;
use spl_pod::solana_msg::msg;

/// Universal authority validation function for all authority types
/// Uses #[track_caller] to provide better error messages with source location
///
/// The fallback is used for mint/freeze authorities which may not be allocated in state yet
/// (e.g., when creating a new compressed mint). Metadata authority never needs fallback
/// because it's always allocated in the TokenMetadata extension (32 bytes, even when revoked).
#[track_caller]
pub fn check_authority(
    current_authority: Option<Pubkey>,
    signer: &pinocchio::pubkey::Pubkey,
    authority_name: &str,
) -> Result<(), ProgramError> {
    // Get authority from current state or fallback to instruction data
    let authority = current_authority.ok_or_else(|| {
        let location = Location::caller();
        msg!(
            "No {} set. {}:{}:{}",
            authority_name,
            location.file(),
            location.line(),
            location.column()
        );
        ErrorCode::InvalidAuthorityMint
    })?;

    // Validate signer matches authority
    if authority.to_bytes() != *signer {
        let location = Location::caller();
        // Check if authority has been revoked (set to zero)
        if authority.to_bytes() == [0u8; 32] {
            msg!(
                "{} has been revoked (set to zero). {}:{}:{}",
                authority_name,
                location.file(),
                location.line(),
                location.column()
            );
            return Err(ErrorCode::InvalidAuthorityMint.into());
        }
        msg!(
            "Invalid {}: signer {:?} doesn't match expected {:?}. {}:{}:{}",
            authority_name,
            solana_pubkey::Pubkey::new_from_array(*signer),
            solana_pubkey::Pubkey::new_from_array(authority.to_bytes()),
            location.file(),
            location.line(),
            location.column()
        );
        return Err(ErrorCode::InvalidAuthorityMint.into());
    }
    Ok(())
}

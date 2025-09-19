use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::Pubkey;
use light_profiler::profile;
use spl_pod::solana_msg::msg;

/// Validates signer authority and updates the authority field in one operation
#[profile]
pub fn validate_and_update_authority(
    authority_field: Option<&Pubkey>,
    instruction_fallback: Option<Pubkey>,
    signer: &pinocchio::pubkey::Pubkey,
    authority_name: &str,
) -> Result<(), ProgramError> {
    // Get current authority (from field or instruction fallback)
    let current_authority = authority_field
        .as_ref()
        .map(|a| **a)
        .or(instruction_fallback)
        .ok_or(ProgramError::InvalidArgument)?;

    // Validate signer matches current authority
    if *signer != current_authority.to_bytes() {
        msg!(
            "Invalid authority: signer does not match current {}",
            authority_name
        );
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

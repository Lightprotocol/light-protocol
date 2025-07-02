use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::Pubkey;
use light_ctoken_types::instructions::mint_actions::ZUpdateAuthority;
use light_zero_copy::traits::ZeroCopyAtMut;
use spl_pod::solana_msg::msg;

/// Validates signer authority and updates the authority field in one operation
pub fn validate_and_update_authority(
    authority_field: &mut <Option<Pubkey> as ZeroCopyAtMut<'_>>::ZeroCopyAtMut,
    instruction_fallback: Option<Pubkey>,
    update_action: &ZUpdateAuthority<'_>,
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

    // Apply update based on allocation and requested change
    let new_authority = update_action.new_authority.as_ref().map(|auth| **auth);
    match (authority_field.as_mut(), new_authority) {
        // Set new authority value in allocated field
        (Some(field_ref), Some(new_auth)) => **field_ref = new_auth,
        // Inconsistent state: allocated Some but trying to revoke
        // This indicates allocation logic bug - revoke should allocate None
        (Some(_), None) => {
            msg!("Zero copy field is some but should be None");
            return Err(ErrorCode::MintActionUnsupportedOperation.into());
        }
        // Invalid operation: cannot set authority when not allocated
        (None, Some(_)) => {
            msg!("Cannot set {} when none was allocated", authority_name);
            return Err(ErrorCode::MintActionUnsupportedOperation.into());
        }
        // Already revoked - no operation needed
        (None, None) => {}
    }

    Ok(())
}

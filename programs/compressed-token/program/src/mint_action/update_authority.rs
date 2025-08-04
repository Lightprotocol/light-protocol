use anchor_lang::solana_program::program_error::ProgramError;

use light_compressed_account::Pubkey;

use spl_pod::solana_msg::msg;

/// Helper function for processing authority update actions
pub fn update_authority(
    update_action: &light_ctoken_types::instructions::mint_actions::ZUpdateAuthority<'_>,
    signer_key: &pinocchio::pubkey::Pubkey,
    current_authority: Option<Pubkey>,
    authority_name: &str,
) -> Result<Option<Pubkey>, ProgramError> {
    // Verify that the signer is the current authority
    let current_authority_pubkey = current_authority.ok_or(ProgramError::InvalidArgument)?;
    if *signer_key != current_authority_pubkey.to_bytes() {
        msg!(
            "Invalid authority: signer does not match current {}",
            authority_name
        );
        return Err(ProgramError::InvalidArgument);
    }

    // Update the authority (None = revoke, Some(key) = set new authority)
    Ok(update_action.new_authority.as_ref().map(|auth| **auth))
}

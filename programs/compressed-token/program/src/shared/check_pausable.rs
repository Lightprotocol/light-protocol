use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::AccountInfoTrait;
use light_token_22::{
    extension::{pausable::PausableConfig, BaseStateWithExtensions, StateWithExtensions},
    state::Mint,
};
use pinocchio::account_info::AccountInfo;

const SPL_TOKEN_2022_ID: [u8; 32] = spl_token_2022::ID.to_bytes();

/// Checks if an SPL Token 2022 mint has the Pausable extension and if it's currently paused.
/// Returns an error if the mint is paused, otherwise Ok(()).
///
/// This function should be called before any token operation (transfer, compress, decompress)
/// when the token account has the PausableAccount extension.
///
/// # Arguments
/// * `mint_account` - The SPL Token 2022 mint account to check
///
/// # Errors
/// * `ErrorCode::MintPaused` - If the mint has PausableConfig and is currently paused
pub fn check_mint_not_paused(mint_account: &AccountInfo) -> Result<(), ProgramError> {
    // Only Token-2022 mints can have the Pausable extension
    if !mint_account.is_owned_by(&SPL_TOKEN_2022_ID) {
        return Ok(());
    }

    let mint_data = AccountInfoTrait::try_borrow_data(mint_account)?;

    // Parse mint with extensions
    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_data)?;

    // Check if mint has PausableConfig extension
    if let Ok(pausable_config) = mint_state.get_extension::<PausableConfig>() {
        // Check if paused
        if bool::from(pausable_config.paused) {
            return Err(ErrorCode::MintPaused.into());
        }
    }

    Ok(())
}

/// Checks if an SPL Token 2022 mint has the Pausable extension.
/// Returns true if the mint has the Pausable extension, false otherwise.
///
/// # Arguments
/// * `mint_account` - The SPL Token 2022 mint account to check
pub fn mint_has_pausable_extension(mint_account: &AccountInfo) -> Result<bool, ProgramError> {
    // Only Token-2022 mints can have the Pausable extension
    if !mint_account.is_owned_by(&SPL_TOKEN_2022_ID) {
        return Ok(false);
    }

    let mint_data = AccountInfoTrait::try_borrow_data(mint_account)?;

    // Parse mint with extensions
    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_data)?;

    // Check if mint has PausableConfig extension
    Ok(mint_state.get_extension::<PausableConfig>().is_ok())
}

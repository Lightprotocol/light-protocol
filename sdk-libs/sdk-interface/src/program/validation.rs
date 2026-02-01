//! Shared validation utilities for compress/decompress operations.

use light_account_checks::{
    account_iterator::AccountIterator, checks::check_data_is_zeroed, AccountInfoTrait,
};

use crate::{error::LightPdaError, program::config::LightConfig};

/// Validated PDA context after account extraction and config validation.
pub struct ValidatedPdaContext<AI: AccountInfoTrait> {
    pub fee_payer: AI,
    pub light_config: LightConfig,
    pub rent_sponsor: AI,
    pub rent_sponsor_bump: u8,
    /// Only present when EXTRACT_COMPRESSION_AUTHORITY=true
    pub compression_authority: Option<AI>,
}

/// Extract and validate accounts for compress operations (4 accounts including compression_authority).
///
/// # Account layout:
/// - `0` - fee_payer (Signer, mut)
/// - `1` - config (LightConfig PDA)
/// - `2` - rent_sponsor (mut)
/// - `3` - compression_authority
pub fn validate_compress_accounts<AI: AccountInfoTrait + Clone>(
    remaining_accounts: &[AI],
    program_id: &[u8; 32],
) -> Result<ValidatedPdaContext<AI>, LightPdaError> {
    validate_pda_common_accounts_inner::<true, AI>(remaining_accounts, program_id)
}

/// Extract and validate accounts for decompress operations (3 accounts, no compression_authority).
///
/// # Account layout:
/// - `0` - fee_payer (Signer, mut)
/// - `1` - config (LightConfig PDA)
/// - `2` - rent_sponsor (mut)
pub fn validate_decompress_accounts<AI: AccountInfoTrait + Clone>(
    remaining_accounts: &[AI],
    program_id: &[u8; 32],
) -> Result<ValidatedPdaContext<AI>, LightPdaError> {
    validate_pda_common_accounts_inner::<false, AI>(remaining_accounts, program_id)
}

/// Internal function with const generic for optional compression_authority extraction.
fn validate_pda_common_accounts_inner<const EXTRACT_COMPRESSION_AUTHORITY: bool, AI>(
    remaining_accounts: &[AI],
    program_id: &[u8; 32],
) -> Result<ValidatedPdaContext<AI>, LightPdaError>
where
    AI: AccountInfoTrait + Clone,
{
    let mut account_iter = AccountIterator::new(remaining_accounts);

    let fee_payer = account_iter
        .next_signer_mut("fee_payer")
        .map_err(LightPdaError::AccountCheck)?;
    let config = account_iter
        .next_non_mut("config")
        .map_err(LightPdaError::AccountCheck)?;
    let rent_sponsor = account_iter
        .next_mut("rent_sponsor")
        .map_err(LightPdaError::AccountCheck)?;

    let compression_authority = if EXTRACT_COMPRESSION_AUTHORITY {
        Some(
            account_iter
                .next_account("compression_authority")
                .map_err(LightPdaError::AccountCheck)?
                .clone(),
        )
    } else {
        None
    };

    let light_config = LightConfig::load_checked(config, program_id)?;

    let rent_sponsor_bump = light_config.validate_rent_sponsor_account(rent_sponsor)?;

    Ok(ValidatedPdaContext {
        fee_payer: fee_payer.clone(),
        light_config,
        rent_sponsor: rent_sponsor.clone(),
        rent_sponsor_bump,
        compression_authority,
    })
}

/// Validate and split remaining_accounts at system_accounts_offset.
///
/// Returns (accounts_before_offset, accounts_from_offset).
pub fn split_at_system_accounts_offset<AI>(
    remaining_accounts: &[AI],
    system_accounts_offset: u8,
) -> Result<(&[AI], &[AI]), LightPdaError> {
    let offset = system_accounts_offset as usize;
    remaining_accounts
        .split_at_checked(offset)
        .ok_or(LightPdaError::ConstraintViolation)
}

/// Extract PDA accounts from the tail of remaining_accounts.
pub fn extract_tail_accounts<AI>(
    remaining_accounts: &[AI],
    num_pda_accounts: usize,
) -> Result<&[AI], LightPdaError> {
    let start = remaining_accounts
        .len()
        .checked_sub(num_pda_accounts)
        .ok_or(LightPdaError::ConstraintViolation)?;
    Ok(&remaining_accounts[start..])
}

/// Check if PDA account is already initialized (has non-zero discriminator).
///
/// Returns:
/// - `Ok(true)` if account has data and non-zero discriminator (initialized)
/// - `Ok(false)` if account is empty or has zeroed discriminator (not initialized)
pub fn is_pda_initialized<AI: AccountInfoTrait>(account: &AI) -> Result<bool, LightPdaError> {
    use light_account_checks::discriminator::DISCRIMINATOR_LEN;

    if account.data_is_empty() {
        return Ok(false);
    }
    let data = account
        .try_borrow_data()
        .map_err(|_| LightPdaError::ConstraintViolation)?;
    if data.len() < DISCRIMINATOR_LEN {
        return Ok(false);
    }
    // If discriminator is NOT zeroed, account is initialized
    Ok(check_data_is_zeroed::<DISCRIMINATOR_LEN>(&data).is_err())
}

/// Check if account should be skipped during compression.
///
/// Returns true if:
/// - Account has no data (empty)
/// - Account is not owned by the expected program
pub fn should_skip_compression<AI: AccountInfoTrait>(
    account: &AI,
    expected_owner: &[u8; 32],
) -> bool {
    account.data_is_empty() || !account.is_owned_by(expected_owner)
}

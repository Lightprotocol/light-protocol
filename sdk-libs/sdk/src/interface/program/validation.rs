//! Shared validation utilities for compress/decompress operations.

use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

use crate::{
    error::LightSdkError,
    interface::LightConfig,
    light_account_checks::{account_iterator::AccountIterator, checks::check_data_is_zeroed},
};

/// Validated PDA context after account extraction and config validation.
pub struct ValidatedPdaContext<'info> {
    pub fee_payer: AccountInfo<'info>,
    pub light_config: LightConfig,
    pub rent_sponsor: AccountInfo<'info>,
    pub rent_sponsor_bump: u8,
    /// Only present when EXTRACT_COMPRESSION_AUTHORITY=true
    pub compression_authority: Option<AccountInfo<'info>>,
}

/// Extract and validate accounts for compress operations (4 accounts including compression_authority).
///
/// # Account layout:
/// - `0` - fee_payer (Signer, mut)
/// - `1` - config (LightConfig PDA)
/// - `2` - rent_sponsor (mut)
/// - `3` - compression_authority (TODO: Signer when client-side code is updated)
pub fn validate_compress_accounts<'info>(
    remaining_accounts: &[AccountInfo<'info>],
    program_id: &Pubkey,
) -> Result<ValidatedPdaContext<'info>, ProgramError> {
    validate_pda_common_accounts_inner::<true>(remaining_accounts, program_id)
}

/// Extract and validate accounts for decompress operations (3 accounts, no compression_authority).
///
/// # Account layout:
/// - `0` - fee_payer (Signer, mut)
/// - `1` - config (LightConfig PDA)
/// - `2` - rent_sponsor (mut)
pub fn validate_decompress_accounts<'info>(
    remaining_accounts: &[AccountInfo<'info>],
    program_id: &Pubkey,
) -> Result<ValidatedPdaContext<'info>, ProgramError> {
    validate_pda_common_accounts_inner::<false>(remaining_accounts, program_id)
}

/// Internal function with const generic for optional compression_authority extraction.
///
/// # Security checks:
/// - fee_payer is signer and mutable
/// - config exists and is not mutable
/// - rent_sponsor is mutable
/// - compression_authority is extracted (if EXTRACT_COMPRESSION_AUTHORITY=true)
/// - LightConfig ownership matches program_id
/// - LightConfig PDA derivation is correct
/// - rent_sponsor matches config.rent_sponsor
/// - TODO: compression_authority matches config.compression_authority (when enabled)
fn validate_pda_common_accounts_inner<'info, const EXTRACT_COMPRESSION_AUTHORITY: bool>(
    remaining_accounts: &[AccountInfo<'info>],
    program_id: &Pubkey,
) -> Result<ValidatedPdaContext<'info>, ProgramError> {
    let mut account_iter = AccountIterator::new(remaining_accounts);

    let fee_payer = account_iter
        .next_signer_mut("fee_payer")
        .map_err(ProgramError::from)?;
    let config = account_iter
        .next_non_mut("config")
        .map_err(ProgramError::from)?;
    let rent_sponsor = account_iter
        .next_mut("rent_sponsor")
        .map_err(ProgramError::from)?;

    let compression_authority = if EXTRACT_COMPRESSION_AUTHORITY {
        // TODO: make compression_authority a signer when client-side code is updated
        Some(
            account_iter
                .next_account("compression_authority")
                .map_err(ProgramError::from)?
                .clone(),
        )
    } else {
        None
    };

    let light_config = LightConfig::load_checked(config, program_id)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    let rent_sponsor_bump = light_config
        .validate_rent_sponsor(rent_sponsor)
        .map_err(|_| LightSdkError::InvalidRentSponsor)?;

    // TODO: validate compression_authority matches config when client-side code is updated
    // if EXTRACT_COMPRESSION_AUTHORITY {
    //     if let Some(ref auth) = compression_authority {
    //         if *auth.key != light_config.compression_authority {
    //             solana_msg::msg!(
    //                 "compression_authority mismatch: expected {:?}, got {:?}",
    //                 light_config.compression_authority,
    //                 auth.key
    //             );
    //             return Err(LightSdkError::ConstraintViolation.into());
    //         }
    //     }
    // }

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
pub fn split_at_system_accounts_offset<'a, 'info>(
    remaining_accounts: &'a [AccountInfo<'info>],
    system_accounts_offset: u8,
) -> Result<(&'a [AccountInfo<'info>], &'a [AccountInfo<'info>]), ProgramError> {
    let offset = system_accounts_offset as usize;
    remaining_accounts.split_at_checked(offset).ok_or_else(|| {
        solana_msg::msg!(
            "system_accounts_offset {} > len {}",
            offset,
            remaining_accounts.len()
        );
        ProgramError::InvalidInstructionData
    })
}

/// Extract PDA accounts from the tail of remaining_accounts.
pub fn extract_tail_accounts<'a, 'info>(
    remaining_accounts: &'a [AccountInfo<'info>],
    num_pda_accounts: usize,
) -> Result<&'a [AccountInfo<'info>], ProgramError> {
    let start = remaining_accounts
        .len()
        .checked_sub(num_pda_accounts)
        .ok_or_else(|| {
            solana_msg::msg!(
                "num_pda_accounts {} > len {}",
                num_pda_accounts,
                remaining_accounts.len()
            );
            ProgramError::NotEnoughAccountKeys
        })?;
    Ok(&remaining_accounts[start..])
}

/// Check if PDA account is already initialized (has non-zero discriminator).
///
/// Returns:
/// - `Ok(true)` if account has data and non-zero discriminator (initialized)
/// - `Ok(false)` if account is empty or has zeroed discriminator (not initialized)
pub fn is_pda_initialized(account: &AccountInfo) -> Result<bool, ProgramError> {
    use crate::light_account_checks::discriminator::DISCRIMINATOR_LEN;

    if account.data_is_empty() {
        return Ok(false);
    }
    let data = account.try_borrow_data()?;
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
pub fn should_skip_compression(account: &AccountInfo, expected_owner: &Pubkey) -> bool {
    account.data_is_empty() || account.owner != expected_owner
}

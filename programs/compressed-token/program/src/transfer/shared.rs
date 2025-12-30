use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use light_ctoken_interface::{
    state::{CToken, ZExtensionStructMut},
    CTokenError, MintExtensionFlags,
};
use light_program_profiler::profile;
use pinocchio::{account_info::AccountInfo, pubkey::pubkey_eq};

use crate::{
    extensions::{check_mint_extensions, MintExtensionChecks},
    shared::{
        convert_program_error,
        transfer_lamports::{multi_transfer_lamports, Transfer},
    },
};

/// Extension information detected from a single account deserialization.
/// Uses `MintExtensionFlags` for T22 extension flags to avoid duplication.
#[derive(Debug, Default)]
struct AccountExtensionInfo {
    /// T22 extension flags (pausable, permanent_delegate, transfer_fee, transfer_hook)
    flags: MintExtensionFlags,
    /// Top-up amount calculated from compression info
    top_up_amount: u64,
    /// Cached decimals from compressible extension (if has_decimals was set)
    decimals: Option<u8>,
}

impl AccountExtensionInfo {
    #[inline(always)]
    fn check_t22_extensions(&self, other: &Self) -> Result<(), ProgramError> {
        if self.flags.has_pausable != other.flags.has_pausable
            || self.flags.has_permanent_delegate != other.flags.has_permanent_delegate
            || self.flags.has_transfer_fee != other.flags.has_transfer_fee
            || self.flags.has_transfer_hook != other.flags.has_transfer_hook
        {
            Err(ProgramError::InvalidInstructionData)
        } else {
            Ok(())
        }
    }
}

/// Account references for transfer operations
pub struct TransferAccounts<'a> {
    pub source: &'a AccountInfo,
    pub destination: &'a AccountInfo,
    pub authority: &'a AccountInfo,
    pub mint: Option<&'a AccountInfo>,
}

/// Process transfer extensions for CTokenTransfer instruction.
/// Restricted extensions are NOT denied (but will fail anyway due to missing mint).
#[inline(always)]
#[profile]
pub fn process_transfer_extensions_transfer(
    transfer_accounts: TransferAccounts,
    max_top_up: u16,
) -> Result<(bool, Option<u8>), ProgramError> {
    process_transfer_extensions(transfer_accounts, max_top_up, false)
}

/// Process transfer extensions for CTokenTransferChecked instruction.
/// Restricted extensions ARE denied - source account must not have restricted T22 extensions.
#[inline(always)]
#[profile]
pub fn process_transfer_extensions_transfer_checked(
    transfer_accounts: TransferAccounts,
    max_top_up: u16,
) -> Result<(bool, Option<u8>), ProgramError> {
    process_transfer_extensions(transfer_accounts, max_top_up, true)
}

/// Process extensions (pausable check, permanent delegate validation, transfer fee withholding)
/// and calculate/execute top-up transfers.
/// Each account is deserialized exactly once. Mint is checked once if any account has extensions.
///
/// # Arguments
/// * `transfer_accounts` - Account references for source, destination, authority, and optional mint
/// * `max_top_up` - Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
/// * `deny_restricted_extensions` - If true, reject source accounts with restricted T22 extensions
///
/// Returns:
/// - `Ok((true, decimals))` - Permanent delegate is validated as authority/signer, skip pinocchio validation
/// - `Ok((false, decimals))` - Use normal pinocchio owner/delegate validation
/// - `decimals` is Some(u8) if source account has cached decimals in compressible extension
#[inline(always)]
#[profile]
fn process_transfer_extensions(
    transfer_accounts: TransferAccounts,
    max_top_up: u16,
    deny_restricted_extensions: bool,
) -> Result<(bool, Option<u8>), ProgramError> {
    let mut current_slot = 0;

    let (sender_info, signer_is_validated) = validate_sender(
        &transfer_accounts,
        &mut current_slot,
        deny_restricted_extensions,
    )?;

    // Process recipient
    let recipient_info = validate_recipient(transfer_accounts.destination, &mut current_slot)?;
    // Sender and recipient must have matching T22 extension markers
    sender_info.check_t22_extensions(&recipient_info)?;

    // Perform compressible top-up if needed
    transfer_top_up(
        &transfer_accounts,
        sender_info.top_up_amount,
        recipient_info.top_up_amount,
        max_top_up,
    )?;

    // Return decimals from sender (source account has the cached decimals)
    Ok((signer_is_validated, sender_info.decimals))
}

#[inline(always)]
fn transfer_top_up(
    transfer_accounts: &TransferAccounts,
    sender_top_up: u64,
    recipient_top_up: u64,
    max_top_up: u16,
) -> Result<(), ProgramError> {
    if sender_top_up > 0 || recipient_top_up > 0 {
        // Check budget if max_top_up is set (non-zero)
        let total_top_up = sender_top_up.saturating_add(recipient_top_up);
        if max_top_up != 0 && total_top_up > max_top_up as u64 {
            return Err(CTokenError::MaxTopUpExceeded.into());
        }

        let transfers = [
            Transfer {
                account: transfer_accounts.source,
                amount: sender_top_up,
            },
            Transfer {
                account: transfer_accounts.destination,
                amount: recipient_top_up,
            },
        ];
        multi_transfer_lamports(transfer_accounts.authority, &transfers)
            .map_err(convert_program_error)
    } else {
        Ok(())
    }
}

fn validate_sender(
    transfer_accounts: &TransferAccounts,
    current_slot: &mut u64,
    deny_restricted_extensions: bool,
) -> Result<(AccountExtensionInfo, bool), ProgramError> {
    // Process sender once
    let sender_info = process_account_extensions(
        transfer_accounts.source,
        current_slot,
        transfer_accounts.mint,
    )?;

    // Get mint checks if any account has extensions (single mint deserialization)
    let mint_checks = if sender_info.flags.has_restricted_extensions() {
        let mint_account = transfer_accounts
            .mint
            .ok_or(ErrorCode::MintRequiredForTransfer)?;
        Some(check_mint_extensions(
            mint_account,
            deny_restricted_extensions,
        )?)
    } else {
        None
    };

    // Validate permanent delegate for sender
    let signer_is_validated =
        validate_permanent_delegate(mint_checks.as_ref(), transfer_accounts.authority)?;

    Ok((sender_info, signer_is_validated))
}

#[inline(always)]
fn validate_recipient(
    account: &AccountInfo,
    current_slot: &mut u64,
) -> Result<AccountExtensionInfo, ProgramError> {
    // No mint validation for recipient - only sender needs to match mint
    process_account_extensions(account, current_slot, None)
}

/// Validate permanent delegate authority.
/// Returns true if authority is the permanent delegate and is a signer.
#[inline(always)]
fn validate_permanent_delegate(
    mint_checks: Option<&MintExtensionChecks>,
    authority: &AccountInfo,
) -> Result<bool, ProgramError> {
    if let Some(checks) = mint_checks {
        if let Some(permanent_delegate_pubkey) = checks.permanent_delegate {
            if pubkey_eq(authority.key(), &permanent_delegate_pubkey) {
                if !authority.is_signer() {
                    return Err(ProgramError::MissingRequiredSignature);
                }
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Process account extensions with mutable access.
/// Performs extension detection and compressible top-up calculation.
/// If mint account is provided, validates it matches the token's mint field.
#[inline(always)]
#[profile]
fn process_account_extensions(
    account: &AccountInfo,
    current_slot: &mut u64,
    mint: Option<&AccountInfo>,
) -> Result<AccountExtensionInfo, ProgramError> {
    let mut account_data = account
        .try_borrow_mut_data()
        .map_err(convert_program_error)?;
    let (token, remaining) = CToken::zero_copy_at_mut_checked(&mut account_data)?;
    if !remaining.is_empty() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate mint account matches token's mint field
    if let Some(mint_account) = mint {
        if !pubkey_eq(mint_account.key(), token.mint.array_ref()) {
            return Err(CTokenError::InvalidAccountData.into());
        }
    }

    let mut info = AccountExtensionInfo::default();

    // Only calculate top-up if account has Compressible extension
    if let Some(compression) = token.get_compressible_extension() {
        // Get current slot for compressible top-up calculation
        use pinocchio::sysvars::{clock::Clock, rent::Rent, Sysvar};
        if *current_slot == 0 {
            *current_slot = Clock::get()
                .map_err(|_| CTokenError::SysvarAccessError)?
                .slot;
        }

        let rent_exemption = Rent::get()
            .map_err(|_| CTokenError::SysvarAccessError)?
            .minimum_balance(account.data_len());

        info.top_up_amount = compression
            .info
            .calculate_top_up_lamports(
                account.data_len() as u64,
                *current_slot,
                account.lamports(),
                rent_exemption,
            )
            .map_err(|_| CTokenError::InvalidAccountData)?;

        // Extract cached decimals if set
        info.decimals = compression.decimals();
    }

    // Process other extensions if present
    if let Some(extensions) = token.extensions {
        for extension in extensions {
            match extension {
                ZExtensionStructMut::PausableAccount(_) => {
                    info.flags.has_pausable = true;
                }
                ZExtensionStructMut::PermanentDelegateAccount(_) => {
                    info.flags.has_permanent_delegate = true;
                }
                ZExtensionStructMut::TransferFeeAccount(_transfer_fee_ext) => {
                    info.flags.has_transfer_fee = true;
                    // Note: Non-zero transfer fees are rejected by check_mint_extensions,
                    // so no fee withholding is needed here.
                }
                ZExtensionStructMut::TransferHookAccount(_) => {
                    info.flags.has_transfer_hook = true;
                    // No runtime logic needed - we only support nil program_id
                }
                ZExtensionStructMut::Compressible(_) => {
                    // Already handled above via get_compressible_extension()
                }
                // Placeholder and TokenMetadata variants are not valid for CToken accounts
                _ => {
                    return Err(CTokenError::InvalidAccountData.into());
                }
            }
        }
    }

    Ok(info)
}

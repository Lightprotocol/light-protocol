use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::{msg, program_error::ProgramError};
use light_program_profiler::profile;
use light_token_interface::{
    state::{Token, ZExtensionStructMut},
    MintExtensionFlags, TokenError,
};
use pinocchio::{account_info::AccountInfo, pubkey::pubkey_eq};

use crate::{
    extensions::{check_mint_extensions, MintExtensionChecks},
    shared::{
        convert_program_error,
        transfer_lamports::{multi_transfer_lamports, Transfer},
    },
};

/// Validates self-transfer: if source == destination, checks authority is signer
/// and is owner or delegate of the token account.
/// Also checks that the account is not frozen and has sufficient funds.
/// Returns Ok(true) if self-transfer was validated (caller should return Ok(())),
/// Returns Ok(false) if not a self-transfer (caller should continue).
#[inline(always)]
pub fn validate_self_transfer(
    source: &AccountInfo,
    destination: &AccountInfo,
    authority: &AccountInfo,
    instruction_data: &[u8],
) -> Result<bool, ProgramError> {
    if !pubkey_eq(source.key(), destination.key()) {
        return Ok(false);
    }
    validate_self_transfer_authority(source, authority, instruction_data)?;
    Ok(true)
}

#[cold]
fn validate_self_transfer_authority(
    source: &AccountInfo,
    authority: &AccountInfo,
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if !authority.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // from_account_info_checked rejects frozen accounts (state != 1)
    let token =
        Token::from_account_info_checked(source).map_err(|_| ProgramError::InvalidAccountData)?;
    let amount = u64::from_le_bytes(
        instruction_data[..8]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );
    if token.base.amount < amount {
        return Err(ErrorCode::InsufficientFunds.into());
    }
    let is_owner = pubkey_eq(authority.key(), token.base.owner.array_ref());
    let is_delegate = token
        .base
        .delegate()
        .is_some_and(|d| pubkey_eq(authority.key(), d.array_ref()));
    if !is_owner && !is_delegate {
        msg!("Self-transfer authority must be owner or delegate");
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

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
            || self.flags.has_default_account_state != other.flags.has_default_account_state
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
    /// Optional fee payer for rent top-ups. If not provided, authority pays.
    pub fee_payer: Option<&'a AccountInfo>,
}

/// Process transfer extensions for CTokenTransfer instruction.
/// Restricted extensions are NOT allowed (requires mint account which is not provided).
#[inline(always)]
#[profile]
pub fn process_transfer_extensions_transfer(
    transfer_accounts: TransferAccounts,
    max_top_up: u16,
) -> Result<(bool, Option<u8>), ProgramError> {
    process_transfer_extensions(transfer_accounts, max_top_up, true)
}

/// Process transfer extensions for CTokenTransferChecked instruction.
/// Restricted extensions are ALLOWED when in valid state - CTokenTransferChecked is the instruction for restricted mints.
#[inline(always)]
#[profile]
pub fn process_transfer_extensions_transfer_checked(
    transfer_accounts: TransferAccounts,
    max_top_up: u16,
) -> Result<(bool, Option<u8>), ProgramError> {
    process_transfer_extensions(transfer_accounts, max_top_up, false)
}

/// Process extensions (pausable check, permanent delegate validation, transfer fee withholding)
/// and calculate/execute top-up transfers.
/// Each account is deserialized exactly once. Mint is checked once if any account has extensions.
///
/// # Arguments
/// * `transfer_accounts` - Account references for source, destination, authority, and optional mint
/// * `max_top_up` - Maximum lamports for top-up. Transaction fails if exceeded. (u16::MAX = no limit, 0 = no top-ups allowed)
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
        // Check budget if limit is set (not u16::MAX)
        // 0 means no top-ups allowed, u16::MAX means no limit
        // max_top_up is in units of 1,000 lamports (max ~65.5M lamports).
        let total_top_up = sender_top_up.saturating_add(recipient_top_up);
        if max_top_up != u16::MAX && total_top_up > (max_top_up as u64).saturating_mul(1000) {
            return Err(TokenError::MaxTopUpExceeded.into());
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
        // Use fee_payer if provided, otherwise fall back to authority
        let payer = transfer_accounts
            .fee_payer
            .unwrap_or(transfer_accounts.authority);
        multi_transfer_lamports(payer, &transfers).map_err(convert_program_error)
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
        // Transfer instruction with ctoken account with restricted extensions will fail here.
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
    let Some(checks) = mint_checks else {
        return Ok(false);
    };
    let Some(permanent_delegate_pubkey) = checks.permanent_delegate else {
        return Ok(false);
    };
    if !pubkey_eq(authority.key(), &permanent_delegate_pubkey) {
        return Ok(false);
    }
    if !authority.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(true)
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
    let token = Token::from_account_info_mut_checked(account)?;

    // Validate mint account matches token's mint field
    if let Some(mint_account) = mint {
        if !pubkey_eq(mint_account.key(), token.mint.array_ref()) {
            return Err(TokenError::InvalidAccountData.into());
        }
    }

    let mut info = AccountExtensionInfo::default();

    // Only calculate top-up if account has Compressible extension
    if let Some(compression) = token.get_compressible_extension() {
        // Get current slot for compressible top-up calculation
        use pinocchio::sysvars::{clock::Clock, Sysvar};
        if *current_slot == 0 {
            *current_slot = Clock::get()
                .map_err(|_| TokenError::SysvarAccessError)?
                .slot;
        }

        info.top_up_amount = compression
            .info
            .calculate_top_up_lamports(account.data_len() as u64, *current_slot, account.lamports())
            .map_err(|_| TokenError::InvalidAccountData)?;

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
                ZExtensionStructMut::TransferFeeAccount(_) => {
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
                    return Err(TokenError::InvalidAccountData.into());
                }
            }
        }
    }

    Ok(info)
}

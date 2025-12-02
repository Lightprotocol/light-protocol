use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::{msg, program_error::ProgramError};
use light_ctoken_types::{
    state::{CToken, ZExtensionStructMut},
    CTokenError,
};
use light_program_profiler::profile;
use pinocchio::{account_info::AccountInfo, pubkey::pubkey_eq};
use pinocchio_token_program::processor::transfer::process_transfer;

use crate::{
    extensions::{check_mint_extensions, MintExtensionChecks},
    shared::{
        convert_program_error,
        transfer_lamports::{multi_transfer_lamports, Transfer},
    },
};

/// Account indices for CToken transfer instruction
const ACCOUNT_SOURCE: usize = 0;
const ACCOUNT_DESTINATION: usize = 1;
const ACCOUNT_AUTHORITY: usize = 2;
const ACCOUNT_MINT: usize = 3;

/// Process ctoken transfer instruction
///
/// Instruction data format (backwards compatible):
/// - 8 bytes: amount (legacy, no max_top_up enforcement)
/// - 10 bytes: amount + max_top_up (u16, 0 = no limit)
#[profile]
#[inline(always)]
pub fn process_ctoken_transfer(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if accounts.len() < 3 {
        msg!(
            "CToken transfer: expected at least 3 accounts received {}",
            accounts.len()
        );
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Validate minimum instruction data length
    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Parse max_top_up based on instruction data length
    // 0 means no limit
    let max_top_up = match instruction_data.len() {
        8 => 0u16, // Legacy: no max_top_up
        10 => u16::from_le_bytes(
            instruction_data[8..10]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        ),
        _ => return Err(ProgramError::InvalidInstructionData),
    };

    let signer_is_validated = process_extensions(accounts, max_top_up)?;

    // Only pass the first 8 bytes (amount) to the SPL transfer processor
    process_transfer(accounts, &instruction_data[..8], signer_is_validated)
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))
}

/// Extension information detected from a single account deserialization
#[derive(Debug, Default)]
struct AccountExtensionInfo {
    has_compressible: bool,
    has_pausable: bool,
    has_permanent_delegate: bool,
    has_transfer_fee: bool,
    has_transfer_hook: bool,
    top_up_amount: u64,
}
impl AccountExtensionInfo {
    fn t22_extensions_eq(&self, other: &Self) -> bool {
        self.has_pausable == other.has_pausable
            && self.has_permanent_delegate == other.has_permanent_delegate
            && self.has_transfer_fee == other.has_transfer_fee
            && self.has_transfer_hook == other.has_transfer_hook
    }

    fn check_t22_extensions(&self, other: &Self) -> Result<(), ProgramError> {
        if !self.t22_extensions_eq(other) {
            Err(ProgramError::InvalidInstructionData)
        } else {
            Ok(())
        }
    }
}

/// Process extensions (pausable check, permanent delegate validation, transfer fee withholding)
/// and calculate/execute top-up transfers.
/// Each account is deserialized exactly once. Mint is checked once if any account has extensions.
///
/// # Arguments
/// * `accounts` - The account infos (source, dest, authority/payer, optional mint)
/// * `max_top_up` - Maximum lamports for rent and top-up combined. Transaction fails if exceeded. (0 = no limit)
///
/// Returns:
/// - `Ok(true)` - Permanent delegate is validated as authority/signer, skip pinocchio validation
/// - `Ok(false)` - Use normal pinocchio owner/delegate validation
#[inline(always)]
#[profile]
fn process_extensions(
    accounts: &[pinocchio::account_info::AccountInfo],
    max_top_up: u16,
) -> Result<bool, ProgramError> {
    let account0 = accounts
        .get(ACCOUNT_SOURCE)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let account1 = accounts
        .get(ACCOUNT_DESTINATION)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let mut current_slot = 0;

    let (sender_info, signer_is_validated) = validate_sender(accounts, &mut current_slot)?;

    // Process recipient
    let recipient_info = validate_recipient(account1, &mut current_slot)?;
    // Sender and recipient must have matching T22 extension markers
    sender_info.check_t22_extensions(&recipient_info)?;

    // Perform compressible top-up if needed
    transfer_top_up(
        accounts,
        account0,
        account1,
        sender_info.top_up_amount,
        recipient_info.top_up_amount,
        max_top_up,
    )?;

    Ok(signer_is_validated)
}

fn transfer_top_up(
    accounts: &[AccountInfo],
    account0: &AccountInfo,
    account1: &AccountInfo,
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

        let payer = accounts
            .get(ACCOUNT_AUTHORITY)
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        let transfers = [
            Transfer {
                account: account0,
                amount: sender_top_up,
            },
            Transfer {
                account: account1,
                amount: recipient_top_up,
            },
        ];
        multi_transfer_lamports(payer, &transfers).map_err(convert_program_error)
    } else {
        Ok(())
    }
}

fn validate_sender(
    accounts: &[AccountInfo],
    current_slot: &mut u64,
) -> Result<(AccountExtensionInfo, bool), ProgramError> {
    let account0 = accounts
        .get(ACCOUNT_SOURCE)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    // Process sender once
    let sender_info = process_account_extensions(account0, current_slot)?;

    // Get mint checks if any account has extensions (single mint deserialization)
    let mint_checks = if sender_info.has_pausable
        || sender_info.has_permanent_delegate
        || sender_info.has_transfer_fee
        || sender_info.has_transfer_hook
    {
        let mint_account = accounts
            .get(ACCOUNT_MINT)
            .ok_or(ErrorCode::MintRequiredForTransfer)?;
        Some(check_mint_extensions(mint_account, false)?)
    } else {
        None
    };

    // Validate permanent delegate for sender
    let signer_is_validated = validate_permanent_delegate(mint_checks.as_ref(), accounts)?;

    Ok((sender_info, signer_is_validated))
}

#[inline(always)]
fn validate_recipient(
    account: &AccountInfo,
    current_slot: &mut u64,
) -> Result<AccountExtensionInfo, ProgramError> {
    process_account_extensions(account, current_slot)
}

/// Validate permanent delegate authority.
/// Returns true if authority is the permanent delegate and is a signer.
#[inline(always)]
fn validate_permanent_delegate(
    mint_checks: Option<&MintExtensionChecks>,
    accounts: &[AccountInfo],
) -> Result<bool, ProgramError> {
    if let Some(checks) = mint_checks {
        if let Some(permanent_delegate_pubkey) = checks.permanent_delegate {
            let authority = accounts
                .get(ACCOUNT_AUTHORITY)
                .ok_or(ProgramError::NotEnoughAccountKeys)?;
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
#[inline(always)]
#[profile]
fn process_account_extensions(
    account: &AccountInfo,
    current_slot: &mut u64,
) -> Result<AccountExtensionInfo, ProgramError> {
    // Fast path: base account with no extensions
    if account.data_len() == light_ctoken_types::BASE_TOKEN_ACCOUNT_SIZE as usize {
        return Ok(AccountExtensionInfo::default());
    }

    let mut account_data = account
        .try_borrow_mut_data()
        .map_err(convert_program_error)?;
    let (token, remaining) = CToken::zero_copy_at_mut_checked(&mut account_data)?;
    if !remaining.is_empty() {
        return Err(ProgramError::InvalidAccountData);
    }

    let extensions = token.extensions.ok_or(CTokenError::InvalidAccountData)?;

    let mut info = AccountExtensionInfo::default();

    for extension in extensions {
        match extension {
            ZExtensionStructMut::Compressible(compressible_extension) => {
                info.has_compressible = true;
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

                info.top_up_amount = compressible_extension
                    .info
                    .calculate_top_up_lamports(
                        account.data_len() as u64,
                        *current_slot,
                        account.lamports(),
                        rent_exemption,
                    )
                    .map_err(|_| CTokenError::InvalidAccountData)?;
            }
            ZExtensionStructMut::PausableAccount(_) => {
                info.has_pausable = true;
            }
            ZExtensionStructMut::PermanentDelegateAccount(_) => {
                info.has_permanent_delegate = true;
            }
            ZExtensionStructMut::TransferFeeAccount(_transfer_fee_ext) => {
                info.has_transfer_fee = true;
                // Note: Non-zero transfer fees are rejected by check_mint_extensions,
                // so no fee withholding is needed here.
            }
            ZExtensionStructMut::TransferHookAccount(_) => {
                info.has_transfer_hook = true;
                // No runtime logic needed - we only support nil program_id
            }
            // Placeholder and TokenMetadata variants are not valid for CToken accounts
            _ => {
                return Err(CTokenError::InvalidAccountData.into());
            }
        }
    }

    Ok(info)
}

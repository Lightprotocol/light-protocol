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

    let signer_is_validated = process_extensions(accounts, instruction_data)?;

    process_transfer(accounts, instruction_data, signer_is_validated)
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
/// Returns:
/// - `Ok(true)` - Permanent delegate is validated as authority/signer, skip pinocchio validation
/// - `Ok(false)` - Use normal pinocchio owner/delegate validation
#[inline(always)]
#[profile]
fn process_extensions(
    accounts: &[pinocchio::account_info::AccountInfo],
    instruction_data: &[u8],
) -> Result<bool, ProgramError> {
    let account0 = accounts
        .get(ACCOUNT_SOURCE)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let account1 = accounts
        .get(ACCOUNT_DESTINATION)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let mut current_slot = 0;

    // Rewritten section
    let (sender_info, mint_checks, signer_is_validated) =
        validate_sender(accounts, instruction_data, &mut current_slot)?;

    // Process recipient once (with mint_checks for fee calculation)
    let recipient_info = validate_recipient(
        account1,
        &mut current_slot,
        mint_checks.as_ref(),
        instruction_data,
    )?;
    // Check sender and recipient extensions are equal
    // TODO: doublec check
    sender_info.check_t22_extensions(&recipient_info)?;

    // Perform compressible top-up if needed
    transfer_top_up(
        accounts,
        account0,
        account1,
        sender_info.top_up_amount,
        recipient_info.top_up_amount,
    )?;

    Ok(signer_is_validated)
}

fn transfer_top_up(
    accounts: &[AccountInfo],
    account0: &AccountInfo,
    account1: &AccountInfo,
    sender_top_up: u64,
    recipient_top_up: u64,
) -> Result<(), ProgramError> {
    if sender_top_up > 0 || recipient_top_up > 0 {
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
    instruction_data: &[u8],
    current_slot: &mut u64,
) -> Result<(AccountExtensionInfo, Option<MintExtensionChecks>, bool), ProgramError> {
    let account0 = accounts
        .get(ACCOUNT_SOURCE)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    // Process sender once
    let sender_info =
        process_account_extensions::<false>(account0, current_slot, None, instruction_data)?;

    // Get mint checks if any account has extensions (single mint deserialization)
    let mint_checks = if sender_info.has_pausable
        || sender_info.has_permanent_delegate
        || sender_info.has_transfer_fee
        || sender_info.has_transfer_hook
    {
        let mint_account = accounts
            .get(ACCOUNT_MINT)
            .ok_or(ErrorCode::MintRequiredForTransfer)?;
        Some(check_mint_extensions(mint_account, true)?)
    } else {
        None
    };

    // Validate permanent delegate for sender
    let signer_is_validated = validate_permanent_delegate(mint_checks.as_ref(), accounts)?;

    Ok((sender_info, mint_checks, signer_is_validated))
}

#[inline(always)]
fn validate_recipient(
    account: &AccountInfo,
    current_slot: &mut u64,
    mint_checks: Option<&MintExtensionChecks>,
    instruction_data: &[u8],
) -> Result<AccountExtensionInfo, ProgramError> {
    process_account_extensions::<true>(account, current_slot, mint_checks, instruction_data)
}

/// Parse transfer amount from instruction data.
/// Format: 8 bytes amount (little-endian), discriminator already stripped
#[inline(always)]
fn parse_transfer_amount(instruction_data: &[u8]) -> Result<u64, ProgramError> {
    if instruction_data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(u64::from_le_bytes(
        instruction_data[0..8]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    ))
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
/// Performs extension detection and:
/// - For recipient (IS_RECIPIENT=true): fee calculation and withholding
#[inline(always)]
#[profile]
fn process_account_extensions<const IS_RECIPIENT: bool>(
    account: &AccountInfo,
    current_slot: &mut u64,
    mint_checks: Option<&MintExtensionChecks>,
    instruction_data: &[u8],
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
                use pinocchio::sysvars::{clock::Clock, Sysvar};
                if *current_slot == 0 {
                    *current_slot = Clock::get()
                        .map_err(|_| CTokenError::SysvarAccessError)?
                        .slot;
                }

                info.top_up_amount = compressible_extension
                    .info
                    .calculate_top_up_lamports(
                        account.data_len() as u64,
                        *current_slot,
                        account.lamports(),
                        compressible_extension.info.lamports_per_write.into(),
                        light_ctoken_types::COMPRESSIBLE_TOKEN_RENT_EXEMPTION,
                    )
                    .map_err(|_| CTokenError::InvalidAccountData)?;
            }
            ZExtensionStructMut::PausableAccount(_) => {
                info.has_pausable = true;
            }
            ZExtensionStructMut::PermanentDelegateAccount(_) => {
                info.has_permanent_delegate = true;
            }
            ZExtensionStructMut::TransferFeeAccount(mut transfer_fee_ext) => {
                info.has_transfer_fee = true;
                // Only calculate and withhold fee on recipient account
                if IS_RECIPIENT {
                    if let Some(checks) = mint_checks {
                        let amount = parse_transfer_amount(instruction_data)?;
                        let fee = checks.calculate_fee(amount);
                        if fee > 0 {
                            transfer_fee_ext
                                .add_withheld_amount(fee)
                                .map_err(|_| ErrorCode::ArithmeticUnderflow)?;
                        }
                    }
                }
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

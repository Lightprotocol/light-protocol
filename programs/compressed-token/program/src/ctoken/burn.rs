use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::{msg, program_error::ProgramError};
use light_account_checks::AccountInfoTrait;
use light_program_profiler::profile;
use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError as PinocchioProgramError,
    pubkey::pubkey_eq,
};
use pinocchio_token_program::processor::{burn::process_burn, burn_checked::process_burn_checked};
use spl_token_2022::{
    extension::{
        permanent_delegate::PermanentDelegate, BaseStateWithExtensions, PodStateWithExtensions,
    },
    pod::PodMint,
};

use crate::shared::{
    compressible_top_up::calculate_and_execute_compressible_top_ups, convert_pinocchio_token_error,
};

pub(crate) type ProcessorFn = fn(&[AccountInfo], &[u8]) -> Result<(), PinocchioProgramError>;

/// Base instruction data length constants
pub(crate) const BASE_LEN_UNCHECKED: usize = 8;
pub(crate) const BASE_LEN_CHECKED: usize = 9;

/// Burn account indices: [ctoken=0, cmint=1, authority=2, system_program=3, fee_payer=4 (optional)]
const BURN_CMINT_IDX: usize = 1;
const BURN_CTOKEN_IDX: usize = 0;
const PAYER_IDX: usize = 2;
#[allow(dead_code)]
const SYSTEM_PROGRAM_IDX: usize = 3;
const FEE_PAYER_IDX: usize = 4;

const SPL_TOKEN_2022_ID: [u8; 32] = spl_token_2022::ID.to_bytes();

/// SPL Token account layout offsets
const TOKEN_AMOUNT_OFFSET: usize = 64;
const MINT_SUPPLY_OFFSET: usize = 36;

/// Process ctoken burn instruction
///
/// Instruction data format (same as CTokenTransfer/CTokenMintTo):
/// - 8 bytes: amount (legacy, no max_top_up enforcement)
/// - 10 bytes: amount + max_top_up (u16, 0 = no limit)
///
/// Account layout:
/// 0: source CToken account (writable)
/// 1: CMint account (writable)
/// 2: authority (signer, readonly if fee_payer provided, writable otherwise)
/// 3: system_program (readonly) - required for rent top-up CPIs
/// 4: fee_payer (optional, signer, writable) - pays for top-ups instead of authority
#[profile]
#[inline(always)]
pub fn process_ctoken_burn(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    process_ctoken_supply_change_inner::<BASE_LEN_UNCHECKED, BURN_CMINT_IDX, BURN_CTOKEN_IDX>(
        accounts,
        instruction_data,
        process_burn,
    )
}

/// Process ctoken burn_checked instruction
///
/// Instruction data format:
/// - 9 bytes: amount (8) + decimals (1) - legacy, no max_top_up enforcement
/// - 11 bytes: amount (8) + decimals (1) + max_top_up (2, u16, 0 = no limit)
///
/// Account layout (same as burn):
/// 0: source CToken account (writable)
/// 1: CMint account (writable)
/// 2: authority (signer, readonly if fee_payer provided, writable otherwise)
/// 3: system_program (readonly) - required for rent top-up CPIs
/// 4: fee_payer (optional, signer, writable) - pays for top-ups instead of authority
#[profile]
#[inline(always)]
pub fn process_ctoken_burn_checked(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    process_ctoken_supply_change_inner::<BASE_LEN_CHECKED, BURN_CMINT_IDX, BURN_CTOKEN_IDX>(
        accounts,
        instruction_data,
        process_burn_checked,
    )
}

/// Try to burn as permanent delegate by checking if the mint is a T22 mint
/// with a PermanentDelegate extension matching the authority.
///
/// Returns Ok(true) if burn was handled as permanent delegate, Ok(false) if not applicable.
/// On true, the CToken balance and mint supply have been updated manually.
#[inline(always)]
fn try_burn_as_permanent_delegate(
    ctoken: &AccountInfo,
    mint: &AccountInfo,
    authority: &AccountInfo,
    amount: u64,
) -> Result<bool, ProgramError> {
    // Only T22 mints can have permanent delegate
    if !mint.is_owned_by(&SPL_TOKEN_2022_ID) {
        return Ok(false);
    }

    // Authority must be a signer
    if !authority.is_signer() {
        return Ok(false);
    }

    // Parse mint for PermanentDelegate extension
    let mint_data = AccountInfoTrait::try_borrow_data(mint)?;
    let mint_state = PodStateWithExtensions::<PodMint>::unpack(&mint_data)?;
    let permanent_delegate = match mint_state.get_extension::<PermanentDelegate>() {
        Ok(ext) => {
            match Option::<solana_pubkey::Pubkey>::from(ext.delegate) {
                Some(delegate) => delegate,
                None => return Ok(false), // Extension exists but delegate is nil
            }
        }
        Err(_) => return Ok(false), // No PermanentDelegate extension
    };

    // Check if authority matches permanent delegate
    if !pubkey_eq(authority.key(), &permanent_delegate.to_bytes()) {
        return Ok(false);
    }
    drop(mint_data);

    // Authority is the permanent delegate — manually perform the burn.
    // Subtract amount from CToken balance (SPL Token layout: amount at offset 64)
    {
        let mut ctoken_data = ctoken
            .try_borrow_mut_data()
            .map_err(|_| ProgramError::AccountBorrowFailed)?;
        if ctoken_data.len() < TOKEN_AMOUNT_OFFSET + 8 {
            return Err(ErrorCode::PermanentDelegateBurnFailed.into());
        }
        let current_balance = u64::from_le_bytes(
            ctoken_data[TOKEN_AMOUNT_OFFSET..TOKEN_AMOUNT_OFFSET + 8]
                .try_into()
                .map_err(|_| ErrorCode::PermanentDelegateBurnFailed)?,
        );
        let new_balance = current_balance
            .checked_sub(amount)
            .ok_or(ErrorCode::PermanentDelegateBurnFailed)?;
        ctoken_data[TOKEN_AMOUNT_OFFSET..TOKEN_AMOUNT_OFFSET + 8]
            .copy_from_slice(&new_balance.to_le_bytes());
    }

    // Subtract amount from mint supply (SPL Token layout: supply at offset 36)
    {
        let mut mint_data = mint
            .try_borrow_mut_data()
            .map_err(|_| ProgramError::AccountBorrowFailed)?;
        if mint_data.len() < MINT_SUPPLY_OFFSET + 8 {
            return Err(ErrorCode::PermanentDelegateBurnFailed.into());
        }
        let current_supply = u64::from_le_bytes(
            mint_data[MINT_SUPPLY_OFFSET..MINT_SUPPLY_OFFSET + 8]
                .try_into()
                .map_err(|_| ErrorCode::PermanentDelegateBurnFailed)?,
        );
        let new_supply = current_supply
            .checked_sub(amount)
            .ok_or(ErrorCode::PermanentDelegateBurnFailed)?;
        mint_data[MINT_SUPPLY_OFFSET..MINT_SUPPLY_OFFSET + 8]
            .copy_from_slice(&new_supply.to_le_bytes());
    }

    Ok(true)
}

/// Shared inner implementation for ctoken mint_to and burn variants.
///
/// # Type Parameters
/// * `BASE_LEN` - Base instruction data length (8 for unchecked, 9 for checked)
/// * `CMINT_IDX` - Index of CMint account (0 for mint_to, 1 for burn)
/// * `CTOKEN_IDX` - Index of CToken account (1 for mint_to, 0 for burn)
///
/// # Arguments
/// * `accounts` - Account layout: [cmint/ctoken, ctoken/cmint, authority]
/// * `instruction_data` - Serialized instruction data
/// * `processor` - Pinocchio processor function
#[inline(always)]
pub(crate) fn process_ctoken_supply_change_inner<
    const BASE_LEN: usize,
    const CMINT_IDX: usize,
    const CTOKEN_IDX: usize,
>(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
    processor: ProcessorFn,
) -> Result<(), ProgramError> {
    if accounts.len() < 3 {
        msg!(
            "CToken: expected at least 3 accounts received {}",
            accounts.len()
        );
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    if instruction_data.len() < BASE_LEN {
        return Err(ProgramError::InvalidInstructionData);
    }

    let max_top_up = match instruction_data.len() {
        len if len == BASE_LEN => 0u16,
        len if len == BASE_LEN + 2 => u16::from_le_bytes(
            instruction_data[BASE_LEN..BASE_LEN + 2]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        ),
        _ => return Err(ProgramError::InvalidInstructionData),
    };

    // For burn operations, check if authority is the permanent delegate.
    // Pinocchio's processor doesn't handle T22 permanent delegate, so we
    // manually perform the burn if the authority matches.
    // Only applicable for burn (CTOKEN_IDX=0, CMINT_IDX=1).
    if CTOKEN_IDX == 0 && CMINT_IDX == 1 {
        let amount = u64::from_le_bytes(
            instruction_data[..8]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        );
        if try_burn_as_permanent_delegate(
            &accounts[CTOKEN_IDX],
            &accounts[CMINT_IDX],
            &accounts[PAYER_IDX],
            amount,
        )? {
            // Burn handled by permanent delegate path, skip pinocchio processor
            let cmint = &accounts[CMINT_IDX];
            let ctoken = &accounts[CTOKEN_IDX];
            let authority_payer = accounts.get(PAYER_IDX);
            let fee_payer = accounts.get(FEE_PAYER_IDX);
            let effective_payer = fee_payer.or(authority_payer);
            return calculate_and_execute_compressible_top_ups(
                cmint,
                ctoken,
                effective_payer,
                max_top_up,
            );
        }
    }

    processor(accounts, &instruction_data[..BASE_LEN]).map_err(convert_pinocchio_token_error)?;

    // Calculate and execute top-ups for both CMint and CToken
    // SAFETY: accounts.len() >= 3 validated at function entry
    let cmint = &accounts[CMINT_IDX];
    let ctoken = &accounts[CTOKEN_IDX];
    // Use fee_payer if provided, otherwise fall back to authority
    let authority_payer = accounts.get(PAYER_IDX);
    let fee_payer = accounts.get(FEE_PAYER_IDX);
    let effective_payer = fee_payer.or(authority_payer);

    calculate_and_execute_compressible_top_ups(cmint, ctoken, effective_payer, max_top_up)
}

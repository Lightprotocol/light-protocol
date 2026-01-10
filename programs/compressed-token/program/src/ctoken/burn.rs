use anchor_lang::solana_program::{msg, program_error::ProgramError};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError as PinocchioProgramError;
use pinocchio_token_program::processor::{burn::process_burn, burn_checked::process_burn_checked};

use crate::shared::{
    compressible_top_up::calculate_and_execute_compressible_top_ups, convert_pinocchio_token_error,
};

pub(crate) type ProcessorFn = fn(&[AccountInfo], &[u8]) -> Result<(), PinocchioProgramError>;

/// Base instruction data length constants
pub(crate) const BASE_LEN_UNCHECKED: usize = 8;
pub(crate) const BASE_LEN_CHECKED: usize = 9;

/// Burn account indices: [ctoken=0, cmint=1, authority=2]
const BURN_CMINT_IDX: usize = 1;
const BURN_CTOKEN_IDX: usize = 0;

/// Process ctoken burn instruction
///
/// Instruction data format (same as CTokenTransfer/CTokenMintTo):
/// - 8 bytes: amount (legacy, no max_top_up enforcement)
/// - 10 bytes: amount + max_top_up (u16, 0 = no limit)
///
/// Account layout:
/// 0: source CToken account (writable)
/// 1: CMint account (writable)
/// 2: authority (signer, also payer for top-ups)
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
/// 2: authority (signer, also payer for top-ups)
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
        msg!("CToken: expected at least 3 accounts received {}", accounts.len());
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

    processor(accounts, &instruction_data[..BASE_LEN]).map_err(convert_pinocchio_token_error)?;

    // Calculate and execute top-ups for both CMint and CToken
    // SAFETY: accounts.len() >= 3 validated at function entry
    let cmint = &accounts[CMINT_IDX];
    let ctoken = &accounts[CTOKEN_IDX];
    let payer = accounts.get(2);

    calculate_and_execute_compressible_top_ups(cmint, ctoken, payer, max_top_up)
}

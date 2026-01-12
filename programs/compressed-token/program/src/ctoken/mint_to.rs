use anchor_lang::solana_program::program_error::ProgramError;
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;
use pinocchio_token_program::processor::{
    mint_to::process_mint_to, mint_to_checked::process_mint_to_checked,
};

use super::burn::{process_ctoken_supply_change_inner, BASE_LEN_CHECKED, BASE_LEN_UNCHECKED};

/// Mint account indices: [cmint=0, ctoken=1, authority=2]
pub(crate) const MINT_CMINT_IDX: usize = 0;
pub(crate) const MINT_CTOKEN_IDX: usize = 1;

/// Process ctoken mint_to instruction
///
/// Instruction data format (same as CTokenTransfer):
/// - 8 bytes: amount (legacy, no max_top_up enforcement)
/// - 10 bytes: amount + max_top_up (u16, 0 = no limit)
///
/// Account layout:
/// 0: CMint account (writable)
/// 1: destination CToken account (writable)
/// 2: authority (signer, also payer for top-ups)
#[profile]
#[inline(always)]
pub fn process_ctoken_mint_to(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    process_ctoken_supply_change_inner::<BASE_LEN_UNCHECKED, MINT_CMINT_IDX, MINT_CTOKEN_IDX>(
        accounts,
        instruction_data,
        process_mint_to,
    )
}

/// Process ctoken mint_to_checked instruction
///
/// Instruction data format:
/// - 9 bytes: amount (8) + decimals (1) - legacy, no max_top_up enforcement
/// - 11 bytes: amount (8) + decimals (1) + max_top_up (2, u16, 0 = no limit)
///
/// Account layout (same as mint_to):
/// 0: CMint account (writable)
/// 1: destination CToken account (writable)
/// 2: authority (signer, also payer for top-ups)
#[profile]
#[inline(always)]
pub fn process_ctoken_mint_to_checked(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    process_ctoken_supply_change_inner::<BASE_LEN_CHECKED, MINT_CMINT_IDX, MINT_CTOKEN_IDX>(
        accounts,
        instruction_data,
        process_mint_to_checked,
    )
}

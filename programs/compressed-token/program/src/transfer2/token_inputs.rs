use anchor_lang::prelude::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressed_account::instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut;
use light_ctoken_types::{
    hash_cache::HashCache, instructions::transfer2::ZCompressedTokenInstructionDataTransfer2,
};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;

use crate::shared::token_input::set_input_compressed_account;

/// Process input compressed accounts and return total input lamports
#[profile]
#[inline(always)]
pub fn set_input_compressed_accounts(
    cpi_instruction_struct: &mut ZInstructionDataInvokeCpiWithReadOnlyMut,
    hash_cache: &mut HashCache,
    inputs: &ZCompressedTokenInstructionDataTransfer2,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
) -> Result<(), ProgramError> {
    for (i, input_data) in inputs.in_token_data.iter().enumerate() {
        let input_lamports = if let Some(lamports) = inputs.in_lamports.as_ref() {
            if let Some(input_lamports) = lamports.get(i) {
                input_lamports.get()
            } else {
                0
            }
        } else {
            0
        };

        set_input_compressed_account(
            cpi_instruction_struct
                .input_compressed_accounts
                .get_mut(i)
                .ok_or(ProgramError::InvalidAccountData)?,
            hash_cache,
            input_data,
            packed_accounts.accounts,
            input_lamports,
        )?;
    }

    Ok(())
}

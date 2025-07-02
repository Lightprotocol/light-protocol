use anchor_lang::prelude::ProgramError;
use light_compressed_account::instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut;
use light_ctoken_types::{
    context::TokenContext, instructions::transfer2::ZCompressedTokenInstructionDataTransfer2,
};

use crate::{
    shared::token_input::set_input_compressed_account, transfer2::accounts::Transfer2PackedAccounts,
};

/// Process input compressed accounts and return total input lamports
pub fn set_input_compressed_accounts(
    cpi_instruction_struct: &mut ZInstructionDataInvokeCpiWithReadOnlyMut,
    context: &mut TokenContext,
    inputs: &ZCompressedTokenInstructionDataTransfer2,
    packed_accounts: &Transfer2PackedAccounts,
) -> Result<u64, ProgramError> {
    let mut total_input_lamports = 0u64;

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

        total_input_lamports += input_lamports;

        set_input_compressed_account::<false>(
            cpi_instruction_struct
                .input_compressed_accounts
                .get_mut(i)
                .ok_or(ProgramError::InvalidAccountData)?,
            context,
            input_data,
            packed_accounts.accounts,
            input_lamports,
        )?;
    }

    Ok(total_input_lamports)
}

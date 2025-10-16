use anchor_lang::prelude::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressed_account::instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut;
use light_ctoken_types::{
    hash_cache::HashCache, instructions::transfer2::ZCompressedTokenInstructionDataTransfer2,
};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;

use crate::shared::token_output::set_output_compressed_account;

/// Process output compressed accounts and return total output lamports
#[profile]
#[inline(always)]
pub fn set_output_compressed_accounts(
    cpi_instruction_struct: &mut ZInstructionDataInvokeCpiWithReadOnlyMut,
    hash_cache: &mut HashCache,
    inputs: &ZCompressedTokenInstructionDataTransfer2,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
) -> Result<(), ProgramError> {
    for (i, output_data) in inputs.out_token_data.iter().enumerate() {
        let output_lamports = if let Some(lamports) = inputs.out_lamports.as_ref() {
            if let Some(lamports) = lamports.get(i) {
                lamports.get()
            } else {
                0
            }
        } else {
            0
        };

        let mint_index = output_data.mint;
        let mint_account = packed_accounts.get_u8(mint_index, "out token mint")?;

        // Get owner account using owner index
        let owner_account = packed_accounts.get_u8(output_data.owner, "out token owner")?;
        let owner_pubkey = *owner_account.key();

        // Get delegate if present
        let delegate_pubkey = if output_data.has_delegate() {
            let delegate_account =
                packed_accounts.get_u8(output_data.delegate, "out token delegete")?;
            Some(*delegate_account.key())
        } else {
            None
        };
        let output_lamports = if output_lamports > 0 {
            Some(output_lamports)
        } else {
            None
        };
        set_output_compressed_account(
            cpi_instruction_struct
                .output_compressed_accounts
                .get_mut(i)
                .ok_or(ProgramError::InvalidAccountData)?,
            hash_cache,
            owner_pubkey.into(),
            delegate_pubkey.map(|d| d.into()),
            output_data.amount,
            output_lamports,
            mint_account.key().into(),
            inputs.output_queue,
            output_data.version,
        )?;
    }

    Ok(())
}

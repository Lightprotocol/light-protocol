use anchor_lang::prelude::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressed_account::instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut;
use light_token_interface::{
    hash_cache::HashCache,
    instructions::{
        extensions::ZExtensionInstructionData, transfer2::ZCompressedTokenInstructionDataTransfer2,
    },
};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;

use super::check_extensions::validate_tlv_and_get_frozen;
use crate::shared::token_output::set_output_compressed_account;

/// Process output compressed accounts
#[profile]
#[inline(always)]
pub fn set_output_compressed_accounts<'a>(
    cpi_instruction_struct: &mut ZInstructionDataInvokeCpiWithReadOnlyMut,
    hash_cache: &mut HashCache,
    inputs: &'a ZCompressedTokenInstructionDataTransfer2<'a>,
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
                packed_accounts.get_u8(output_data.delegate, "out token delegate")?;
            Some(*delegate_account.key())
        } else {
            None
        };
        let output_lamports = if output_lamports > 0 {
            Some(output_lamports)
        } else {
            None
        };

        // Get TLV data for this output
        let tlv_data: Option<&[ZExtensionInstructionData]> = inputs
            .out_tlv
            .as_ref()
            .and_then(|tlvs| tlvs.get(i).map(|ext_vec| ext_vec.as_slice()));

        let is_frozen = validate_tlv_and_get_frozen(tlv_data, output_data.version)?;

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
            tlv_data,
            is_frozen,
        )?;
    }

    Ok(())
}

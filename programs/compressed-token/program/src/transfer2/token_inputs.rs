use anchor_lang::prelude::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressed_account::instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut;
use light_ctoken_interface::{
    hash_cache::HashCache,
    instructions::{
        extensions::ZExtensionInstructionData, transfer2::ZCompressedTokenInstructionDataTransfer2,
    },
};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;

use super::check_extensions::{validate_tlv_and_get_frozen, MintExtensionCache};
use crate::shared::token_input::set_input_compressed_account;

/// Process input compressed accounts and return total input lamports
#[profile]
#[inline(always)]
pub fn set_input_compressed_accounts<'a>(
    cpi_instruction_struct: &mut ZInstructionDataInvokeCpiWithReadOnlyMut,
    hash_cache: &mut HashCache,
    inputs: &'a ZCompressedTokenInstructionDataTransfer2<'a>,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    all_accounts: &[AccountInfo],
    mint_cache: &'a MintExtensionCache,
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

        // Get TLV data for this input
        let tlv_data: Option<&[ZExtensionInstructionData]> = inputs
            .in_tlv
            .as_ref()
            .and_then(|tlvs| tlvs.get(i).map(|ext_vec| ext_vec.as_slice()));

        let is_frozen = validate_tlv_and_get_frozen(tlv_data, input_data.version)?;

        set_input_compressed_account(
            cpi_instruction_struct
                .input_compressed_accounts
                .get_mut(i)
                .ok_or(ProgramError::InvalidAccountData)?,
            hash_cache,
            input_data,
            packed_accounts.accounts,
            all_accounts,
            input_lamports,
            tlv_data,
            mint_cache,
            is_frozen,
        )?;
    }

    Ok(())
}

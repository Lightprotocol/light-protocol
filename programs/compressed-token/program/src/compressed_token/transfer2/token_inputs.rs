use anchor_lang::prelude::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressed_account::instruction_data::with_readonly::ZInstructionDataInvokeCpiWithReadOnlyMut;
use light_ctoken_interface::{
    hash_cache::HashCache,
    instructions::{
        extensions::ZExtensionInstructionData, transfer2::ZCompressedTokenInstructionDataTransfer2,
    },
    CTokenError,
};
use light_program_profiler::profile;
use pinocchio::account_info::AccountInfo;

use super::check_extensions::{validate_tlv_and_get_frozen, MintExtensionCache};
use crate::{shared::token_input::set_input_compressed_account, MAX_COMPRESSIONS};

/// Process input compressed accounts and return compression-to-input lookup.
/// Returns `[Option<u8>; MAX_COMPRESSIONS]` where `compression_to_input[compression_idx] = Some(input_idx)`.
#[profile]
#[inline(always)]
pub fn set_input_compressed_accounts<'a>(
    cpi_instruction_struct: &mut ZInstructionDataInvokeCpiWithReadOnlyMut,
    hash_cache: &mut HashCache,
    inputs: &'a ZCompressedTokenInstructionDataTransfer2<'a>,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    all_accounts: &[AccountInfo],
    mint_cache: &'a MintExtensionCache,
) -> Result<[Option<u8>; MAX_COMPRESSIONS], ProgramError> {
    // compression_to_input[compression_index] = Some(input_index), None means unset
    let mut compression_to_input: [Option<u8>; MAX_COMPRESSIONS] = [None; MAX_COMPRESSIONS];

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

        // Extract compression_index from CompressedOnly TLV if present
        if let Some(tlv) = tlv_data {
            for ext in tlv {
                if let ZExtensionInstructionData::CompressedOnly(co) = ext {
                    let idx = co.compression_index as usize;
                    // TODO check that it is not out of bounds
                    // Check uniqueness - error if compression_index already used
                    if compression_to_input[idx].is_some() {
                        return Err(CTokenError::DuplicateCompressionIndex.into());
                    }
                    compression_to_input[idx] = Some(i as u8);
                }
            }
        }

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

    Ok(compression_to_input)
}

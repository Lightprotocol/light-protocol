use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnlyConfig;
use light_ctoken_types::{
    instructions::{
        extensions::ZExtensionInstructionData, transfer2::ZCompressedTokenInstructionDataTransfer2,
    },
    state::{ExtensionStructConfig, TokenData, TokenDataConfig},
};
use light_program_profiler::profile;
use light_zero_copy::ZeroCopyNew;
use pinocchio::program_error::ProgramError;
use tinyvec::ArrayVec;

use crate::shared::cpi_bytes_size::{
    self, allocate_invoke_with_read_only_cpi_bytes, compressed_token_data_len, cpi_bytes_config,
    CpiConfigInput,
};

/// Build CPI configuration from instruction data
#[profile]
#[inline(always)]
pub fn allocate_cpi_bytes(
    inputs: &ZCompressedTokenInstructionDataTransfer2,
) -> Result<(Vec<u8>, InstructionDataInvokeCpiWithReadOnlyConfig), ProgramError> {
    // Build CPI configuration based on delegate flags
    let mut input_delegate_flags: ArrayVec<[bool; cpi_bytes_size::MAX_INPUT_ACCOUNTS]> =
        ArrayVec::new();
    for input_data in inputs.in_token_data.iter() {
        input_delegate_flags.push(input_data.has_delegate());
    }

    let mut output_accounts = ArrayVec::new();
    for (i, output_data) in inputs.out_token_data.iter().enumerate() {
        let has_delegate = output_data.has_delegate();

        // Check if there's TLV data for this output
        let tlv_data: Option<&[ZExtensionInstructionData]> = inputs
            .out_tlv
            .as_ref()
            .and_then(|tlvs| tlvs.get(i).map(|ext_vec| ext_vec.as_slice()));

        let data_len = if let Some(tlv) = tlv_data {
            if !tlv.is_empty() {
                // Build TLV config for byte length calculation
                let tlv_config: Vec<ExtensionStructConfig> = tlv
                    .iter()
                    .filter_map(|ext| match ext {
                        ZExtensionInstructionData::CompressedOnly(_) => {
                            Some(ExtensionStructConfig::CompressedOnly(()))
                        }
                        _ => None,
                    })
                    .collect();

                let token_config = TokenDataConfig {
                    delegate: (has_delegate, ()),
                    tlv: (true, tlv_config),
                };
                TokenData::byte_len(&token_config).map_err(|_| ProgramError::InvalidAccountData)?
                    as u32
            } else {
                compressed_token_data_len(has_delegate)
            }
        } else {
            compressed_token_data_len(has_delegate)
        };

        output_accounts.push((false, data_len)); // Token accounts don't have addresses
    }

    // Add extra output account for change account if needed (no delegate, no token data)
    if inputs.with_lamports_change_account_merkle_tree_index != 0 {
        output_accounts.push((false, compressed_token_data_len(false)));
        // No delegate
    }

    let mut input_accounts = ArrayVec::new();
    for _ in input_delegate_flags {
        input_accounts.push(false); // Token accounts don't have addresses
    }

    let config_input = CpiConfigInput {
        input_accounts,
        output_accounts,
        has_proof: inputs.proof.is_some(),
        new_address_params: 0, // No new addresses for transfer2
    };
    let config = cpi_bytes_config(config_input);
    Ok((allocate_invoke_with_read_only_cpi_bytes(&config)?, config))
}

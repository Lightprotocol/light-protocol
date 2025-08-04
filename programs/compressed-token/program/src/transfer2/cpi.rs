use arrayvec::ArrayVec;
use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnlyConfig;
use light_ctoken_types::instructions::transfer2::ZCompressedTokenInstructionDataTransfer2;

use crate::shared::cpi_bytes_size::{
    allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
};

/// Build CPI configuration from instruction data
pub fn allocate_cpi_bytes(
    inputs: &ZCompressedTokenInstructionDataTransfer2,
) -> (Vec<u8>, InstructionDataInvokeCpiWithReadOnlyConfig) {
    // Build CPI configuration based on delegate flags
    let mut input_delegate_flags: ArrayVec<bool, 8> = ArrayVec::new();
    for input_data in inputs.in_token_data.iter() {
        input_delegate_flags.push(input_data.with_delegate != 0);
    }

    let mut output_accounts = ArrayVec::new();
    for output_data in inputs.out_token_data.iter() {
        // Check if output has delegate (delegate index != 0 means delegate is present)
        let has_delegate = output_data.delegate != 0;
        output_accounts.push((false, crate::shared::cpi_bytes_size::token_data_len(has_delegate))); // Token accounts don't have addresses
    }

    // Add extra output account for change account if needed (no delegate, no token data)
    if inputs.with_lamports_change_account_merkle_tree_index != 0 {
        output_accounts.push((false, crate::shared::cpi_bytes_size::token_data_len(false))); // No delegate
    }

    let mut input_accounts = ArrayVec::new();
    for has_delegate in input_delegate_flags {
        input_accounts.push(false); // Token accounts don't have addresses
    }

    let config_input = CpiConfigInput {
        input_accounts,
        output_accounts,
        has_proof: inputs.proof.is_some(),
        new_address_params: 0, // No new addresses for transfer2
    };
    let config = cpi_bytes_config(config_input);
    (allocate_invoke_with_read_only_cpi_bytes(&config), config)
}

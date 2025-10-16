use anchor_compressed_token::ErrorCode;
use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnlyConfig;
use light_ctoken_types::{
    instructions::mint_action::{ZAction, ZMintActionCompressedInstructionData},
    state::CompressedMintConfig,
};
use light_program_profiler::profile;
use spl_pod::solana_msg::msg;
use tinyvec::ArrayVec;

use crate::shared::{
    convert_program_error,
    cpi_bytes_size::{
        allocate_invoke_with_read_only_cpi_bytes, compressed_token_data_len, cpi_bytes_config,
        mint_data_len, CpiConfigInput,
    },
};

#[profile]
pub fn get_zero_copy_configs(
    parsed_instruction_data: &mut ZMintActionCompressedInstructionData<'_>,
) -> Result<
    (
        InstructionDataInvokeCpiWithReadOnlyConfig,
        Vec<u8>,
        CompressedMintConfig,
    ),
    ProgramError,
> {
    // Generate output config based on final state after all actions (without modifying instruction data)
    let (_, output_extensions_config, _) =
        crate::extensions::process_extensions_config_with_actions(
            parsed_instruction_data.mint.extensions.as_ref(),
            &parsed_instruction_data.actions,
        )?;
    // Process actions to determine final output state (no instruction data modification)
    for action in parsed_instruction_data.actions.iter() {
        match action {
            ZAction::UpdateMintAuthority(_) => {}
            ZAction::UpdateFreezeAuthority(_) => {}
            ZAction::RemoveMetadataKey(_) => {}
            ZAction::UpdateMetadataAuthority(auth_action) => {
                // Update output config for authority revocation
                if auth_action.new_authority.to_bytes() == [0u8; 32] {
                    let extension_index = auth_action.extension_index as usize;
                    if extension_index >= output_extensions_config.len() {
                        msg!("Extension index {} out of bounds", extension_index);
                        return Err(
                            anchor_compressed_token::ErrorCode::MintActionInvalidExtensionIndex
                                .into(),
                        );
                    }
                }
            }
            _ => {}
        }
    }

    // Output mint config (always present) with final authority states
    let output_mint_config = CompressedMintConfig {
        base: (),
        metadata: (),
        extensions: (
            !output_extensions_config.is_empty(),
            output_extensions_config,
        ),
    };

    // Count recipients from MintTo actions
    let num_recipients = parsed_instruction_data
        .actions
        .iter()
        .map(|action| match action {
            ZAction::MintToCompressed(mint_to_action) => mint_to_action.recipients.len(),
            _ => 0,
        })
        .sum();
    if num_recipients > 29 {
        msg!("Max allowed is 29 compressed token recipients");
        return Err(ErrorCode::TooManyMintToRecipients.into());
    }
    let input = CpiConfigInput {
        input_accounts: {
            let mut inputs = ArrayVec::new();
            // Add input mint if not creating mint
            if parsed_instruction_data.create_mint.is_none() {
                inputs.push(true); // Input mint has address
            }
            inputs
        },
        output_accounts: {
            let mut outputs = ArrayVec::new();
            // First output is always the mint account
            outputs.push((true, mint_data_len(&output_mint_config)));

            // Add token accounts for recipients
            for _ in 0..num_recipients {
                outputs.push((false, compressed_token_data_len(false)));
                // No delegates for simple mint
            }
            outputs
        },
        has_proof: parsed_instruction_data.proof.is_some(),
        // Add new address params if creating a mint
        new_address_params: if parsed_instruction_data.create_mint.is_some() {
            1
        } else {
            0
        },
    };

    let config = cpi_bytes_config(input);
    let cpi_bytes =
        allocate_invoke_with_read_only_cpi_bytes(&config).map_err(convert_program_error)?;

    Ok((config, cpi_bytes, output_mint_config))
}

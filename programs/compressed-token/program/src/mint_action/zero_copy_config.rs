use anchor_lang::solana_program::program_error::ProgramError;
use arrayvec::ArrayVec;
use light_compressed_account::instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnlyConfig;
use light_ctoken_types::{
    instructions::mint_actions::{ZAction, ZMintActionCompressedInstructionData},
    state::CompressedMintConfig,
};

use spl_pod::solana_msg::msg;

use crate::shared::cpi_bytes_size::{
    allocate_invoke_with_read_only_cpi_bytes, cpi_bytes_config, CpiConfigInput,
};

pub fn get_zero_copy_configs(
    parsed_instruction_data: &ZMintActionCompressedInstructionData<'_>,
) -> Result<
    (
        InstructionDataInvokeCpiWithReadOnlyConfig,
        Vec<u8>,
        CompressedMintConfig,
    ),
    ProgramError,
> {
    use light_ctoken_types::state::CompressedMintConfig;
    msg!("get_zero_copy_configs");
    // Process extensions to get the proper config for CPI bytes allocation
    let (_, extensions_config, _) = crate::extensions::process_extensions_config(
        parsed_instruction_data.mint.extensions.as_ref(),
    )?;
    msg!("get_zero_copy_configs1");

    // Calculate final authority states after processing all actions
    let mut final_mint_authority = parsed_instruction_data.mint.mint_authority.is_some();
    let mut final_freeze_authority = parsed_instruction_data.mint.freeze_authority.is_some();

    // Process actions in order to determine final authority states
    for action in parsed_instruction_data.actions.iter() {
        match action {
            ZAction::UpdateMintAuthority(update_action) => {
                // None = revoke authority, Some(key) = set new authority
                final_mint_authority = update_action.new_authority.is_some();
            }
            ZAction::UpdateFreezeAuthority(update_action) => {
                // None = revoke authority, Some(key) = set new authority
                final_freeze_authority = update_action.new_authority.is_some();
            }
            ZAction::UpdateMetadata => {
                // TODO: When UpdateMetadata is implemented, process extension modifications here
                // and recalculate final extensions_config for correct output mint size calculation
            }
            _ => {} // Other actions don't affect authority or extension states
        }
    }
    msg!("get_zero_copy_configs2");

    // Output mint config (always present) with final authority states
    let output_mint_config = CompressedMintConfig {
        mint_authority: (final_mint_authority, ()),
        freeze_authority: (final_freeze_authority, ()),
        extensions: (!extensions_config.is_empty(), extensions_config),
    };

    // Count recipients from MintTo actions
    let num_recipients = parsed_instruction_data
        .actions
        .iter()
        .map(|action| match action {
            ZAction::MintTo(mint_to_action) => mint_to_action.recipients.len(),
            _ => 0,
        })
        .sum();
    msg!("get_zero_copy_configs2");

    let input = CpiConfigInput {
        input_accounts: {
            let mut inputs = ArrayVec::new();
            // Add input mint if not creating mint
            if !parsed_instruction_data.create_mint() {
                inputs.push(true); // Input mint has address
            }
            inputs
        },
        output_accounts: {
            let mut outputs = ArrayVec::new();
            // First output is always the mint account
            outputs.push((
                true,
                crate::shared::cpi_bytes_size::mint_data_len(&output_mint_config),
            ));

            // Add token accounts for recipients
            for _ in 0..num_recipients {
                outputs.push((false, crate::shared::cpi_bytes_size::token_data_len(false)));
                // No delegates for simple mint
            }
            outputs
        },
        has_proof: parsed_instruction_data.proof.is_some(),
        // Add new address params if creating a mint
        new_address_params: if parsed_instruction_data.create_mint() {
            1
        } else {
            0
        },
    };
    msg!("get_zero_copy_configs5");

    let config = cpi_bytes_config(input);
    msg!("get_zero_copy_configs6");
    let cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);
    msg!("get_zero_copy_configs7");

    Ok((config, cpi_bytes, output_mint_config))
}

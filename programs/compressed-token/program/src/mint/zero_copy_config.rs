use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::{
    compressed_account::{CompressedAccountConfig, CompressedAccountDataConfig},
    instruction_data::{
        compressed_proof::CompressedProofConfig, cpi_context::CompressedCpiContextConfig,
        data::OutputCompressedAccountWithPackedContextConfig,
        with_readonly::InstructionDataInvokeCpiWithReadOnlyConfig,
    },
};
use light_ctoken_types::state::{CompressedMint, CompressedMintConfig};
use light_sdk_pinocchio::NewAddressParamsAssignedPackedConfig;
use light_zero_copy::ZeroCopyNew;

// TODO: unit test.
pub fn get_zero_copy_configs(
    parsed_instruction_data: &light_ctoken_types::instructions::create_compressed_mint::ZCreateCompressedMintInstructionData<'_>,
) -> Result<
    (
        CompressedMintConfig,
        InstructionDataInvokeCpiWithReadOnlyConfig,
    ),
    ProgramError,
> {
    let (compressed_mint_len, mint_size_config) = {
        let (has_extensions, extensions_config, additional_mint_data_len) =
            crate::extensions::process_extensions_config(
                parsed_instruction_data.extensions.as_ref(),
            )?;
        let mint_size_config: <CompressedMint as ZeroCopyNew>::ZeroCopyConfig =
            CompressedMintConfig {
                mint_authority: (true, ()),
                freeze_authority: (parsed_instruction_data.freeze_authority.is_some(), ()),
                extensions: (has_extensions, extensions_config),
            };
        (
            (CompressedMint::byte_len(&mint_size_config) + additional_mint_data_len) as u32,
            mint_size_config,
        )
    };
    let output_compressed_accounts = vec![OutputCompressedAccountWithPackedContextConfig {
        compressed_account: CompressedAccountConfig {
            address: (true, ()),
            data: (
                true,
                CompressedAccountDataConfig {
                    data: compressed_mint_len,
                },
            ),
        },
    }];
    let new_address_params = vec![NewAddressParamsAssignedPackedConfig {}];
    let config = InstructionDataInvokeCpiWithReadOnlyConfig {
        cpi_context: CompressedCpiContextConfig {},
        input_compressed_accounts: vec![],
        // We always need a proof to create the compressed address.
        proof: (
            parsed_instruction_data.proof.is_some(),
            CompressedProofConfig {},
        ),
        read_only_accounts: vec![],
        read_only_addresses: vec![],
        new_address_params,
        output_compressed_accounts,
    };
    Ok((mint_size_config, config))
}

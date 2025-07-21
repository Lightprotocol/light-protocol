use anchor_lang::{prelude::msg, solana_program::program_error::ProgramError};
use light_compressed_account::{
    compressed_account::{CompressedAccountConfig, CompressedAccountDataConfig},
    instruction_data::{
        compressed_proof::CompressedProofConfig,
        cpi_context::CompressedCpiContextConfig,
        data::OutputCompressedAccountWithPackedContextConfig,
        with_readonly::{
            InstructionDataInvokeCpiWithReadOnly, InstructionDataInvokeCpiWithReadOnlyConfig,
        },
    },
    Pubkey,
};
use light_sdk_pinocchio::NewAddressParamsAssignedPackedConfig;
use light_zero_copy::{borsh::Deserialize, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;
use spl_token::solana_program::log::sol_log_compute_units;

use crate::{
    mint::{accounts::CreateCompressedMintAccounts, output::create_output_compressed_mint_account},
    shared::cpi::execute_cpi_invoke,
};
use light_ctoken_types::{
    instructions::create_compressed_mint::CreateCompressedMintInstructionData,
    state::{CompressedMint, CompressedMintConfig},
};

pub fn process_create_compressed_mint(
    program_id: pinocchio::pubkey::Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    sol_log_compute_units();
    let (parsed_instruction_data, _) =
        CreateCompressedMintInstructionData::zero_copy_at(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;
    sol_log_compute_units();

    // Validate and parse accounts
    let validated_accounts =
        CreateCompressedMintAccounts::validate_and_parse(accounts, &program_id)?;

    // 1. Create mint PDA using provided bump
    let mint_pda: Pubkey = solana_pubkey::Pubkey::create_program_address(
        &[
            b"compressed_mint",
            validated_accounts.mint_signer.key().as_slice(),
            &[parsed_instruction_data.mint_bump],
        ],
        &program_id.into(),
    )?
    .into();

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
        proof: (true, CompressedProofConfig {}),
        read_only_accounts: vec![],
        read_only_addresses: vec![],
        new_address_params,
        output_compressed_accounts,
    };

    let vec_len = InstructionDataInvokeCpiWithReadOnly::byte_len(&config);
    msg!("vec len {}", vec_len);
    // + discriminator len + vector len
    let mut cpi_bytes = vec![0u8; vec_len + 8];
    cpi_bytes[0..8].copy_from_slice(
        &light_compressed_account::discriminators::DISCRIMINATOR_INVOKE_CPI_WITH_READ_ONLY,
    );

    sol_log_compute_units();
    let (mut cpi_instruction_struct, _) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
            .map_err(ProgramError::from)?;

    cpi_instruction_struct.bump = crate::LIGHT_CPI_SIGNER.bump;
    cpi_instruction_struct.invoking_program_id = crate::LIGHT_CPI_SIGNER.program_id.into();

    sol_log_compute_units();

    let proof = cpi_instruction_struct
        .proof
        .as_deref_mut()
        .ok_or(ProgramError::InvalidInstructionData)?;
    proof.a = parsed_instruction_data.proof.a;
    proof.b = parsed_instruction_data.proof.b;
    proof.c = parsed_instruction_data.proof.c;
    // 1. Create NewAddressParams
    cpi_instruction_struct.new_address_params[0].seed = mint_pda.to_bytes();
    cpi_instruction_struct.new_address_params[0].address_merkle_tree_root_index =
        *parsed_instruction_data.address_merkle_tree_root_index;
    cpi_instruction_struct.new_address_params[0].assigned_account_index = 0;
    // Note we can skip address derivation since we are assigning it to the account in index 0.
    cpi_instruction_struct.new_address_params[0].assigned_to_account = 1;

    // 2. Create compressed mint account data
    // TODO: add input struct, try to use CompressedMintInput
    create_output_compressed_mint_account(
        &mut cpi_instruction_struct.output_compressed_accounts[0],
        mint_pda,
        parsed_instruction_data.decimals,
        parsed_instruction_data.freeze_authority.map(|fa| *fa),
        Some(parsed_instruction_data.mint_authority),
        0.into(),
        &program_id.into(),
        mint_size_config,
        *parsed_instruction_data.mint_address,
        1,
        parsed_instruction_data.version,
        false, // Set is_decompressed = false for new mint creation
        parsed_instruction_data.extensions.as_deref(),
    )?;
    sol_log_compute_units();
    // 4. Execute CPI to light-system-program
    // Extract tree accounts for the generalized CPI call
    let tree_accounts = [accounts[10].key(), accounts[11].key()]; // address_merkle_tree, output_queue

    execute_cpi_invoke(
        &accounts[2..], // Skip two non-CPI account (light system program mint_signer)
        cpi_bytes,
        tree_accounts.as_slice(),
        false, // no sol_pool_pda for create_compressed_mint
        None,  // no cpi_context_account for create_compressed_mint
    )
}

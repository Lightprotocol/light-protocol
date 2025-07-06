use anchor_lang::{
    prelude::msg,
    solana_program::{account_info::AccountInfo, program_error::ProgramError},
};
use light_compressed_account::{
    address::derive_address,
    compressed_account::{CompressedAccountConfig, CompressedAccountDataConfig},
    instruction_data::{
        compressed_proof::CompressedProofConfig,
        cpi_context::CompressedCpiContextConfig,
        data::{NewAddressParamsPackedConfig, OutputCompressedAccountWithPackedContextConfig},
        invoke_cpi::{InstructionDataInvokeCpi, InstructionDataInvokeCpiConfig},
    },
    Pubkey,
};
use light_zero_copy::borsh::Deserialize;
use spl_token::solana_program::log::sol_log_compute_units;

use crate::{
    mint::{
        accounts::CreateCompressedMintAccounts,
        instructions::CreateCompressedMintInstructionData,
        output::create_output_compressed_mint_account,
        state::{CompressedMint, CompressedMintConfig},
    },
    shared::cpi::execute_cpi_invoke,
};

pub fn process_create_compressed_mint<'info>(
    program_id: Pubkey,
    accounts: &'info [AccountInfo<'info>],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    sol_log_compute_units();
    let (parsed_instruction_data, _) =
        CreateCompressedMintInstructionData::zero_copy_at(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;
    sol_log_compute_units();

    // Validate and parse accounts
    let validated_accounts =
        CreateCompressedMintAccounts::validate_and_parse(accounts, &program_id.into())?;
    // 1. Create mint PDA using provided bump
    let mint_pda: Pubkey = solana_pubkey::Pubkey::create_program_address(
        &[
            b"compressed_mint",
            validated_accounts.mint_signer.key.as_ref(),
            &[parsed_instruction_data.mint_bump],
        ],
        &program_id.into(),
    )?
    .into();
    use light_zero_copy::ZeroCopyNew;

    let mint_size_config: <CompressedMint as ZeroCopyNew>::ZeroCopyConfig = CompressedMintConfig {
        mint_authority: (true, ()),
        freeze_authority: (parsed_instruction_data.freeze_authority.is_some(), ()),
    };

    let config = InstructionDataInvokeCpiConfig {
        compress_or_decompress_lamports: false,
        cpi_context: (false, CompressedCpiContextConfig {}),
        input_compressed_accounts_with_merkle_context: vec![],
        proof: (true, CompressedProofConfig {}),
        relay_fee: false,
        new_address_params: vec![NewAddressParamsPackedConfig {}],
        output_compressed_accounts: vec![OutputCompressedAccountWithPackedContextConfig {
            compressed_account: CompressedAccountConfig {
                address: (true, ()),
                data: (
                    true,
                    CompressedAccountDataConfig {
                        data: CompressedMint::byte_len(&mint_size_config) as u32,
                    },
                ),
            },
        }],
    };
    // TODO: InstructionDataInvokeCpi::Output -> InstructionDataInvokeCpi::ZeroCopyMut and InstructionDataInvokeCpi::ZeroCopy
    // TODO: hardcode since len is constant
    let vec_len = InstructionDataInvokeCpi::byte_len(&config);
    msg!("vec len {}", vec_len);
    // + discriminator len + vector len
    let mut cpi_bytes = vec![0u8; vec_len + 8 + 4];
    cpi_bytes[0..8]
        .copy_from_slice(&light_compressed_account::discriminators::DISCRIMINATOR_INVOKE_CPI);
    cpi_bytes[8..12].copy_from_slice(&(vec_len as u32).to_le_bytes());

    sol_log_compute_units();
    let (mut cpi_instruction_struct, _) =
        InstructionDataInvokeCpi::new_zero_copy(&mut cpi_bytes[12..], config)
            .map_err(ProgramError::from)?;
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

    // 2. Derive compressed account address
    let compressed_account_address = derive_address(
        &mint_pda.to_bytes(),
        &validated_accounts.address_merkle_tree.key.to_bytes(),
        &program_id.to_bytes(),
    );

    // 2. Create compressed mint account data
    create_output_compressed_mint_account(
        &mut cpi_instruction_struct.output_compressed_accounts[0],
        mint_pda,
        parsed_instruction_data.decimals,
        parsed_instruction_data.freeze_authority.map(|fa| *fa),
        Some(parsed_instruction_data.mint_authority),
        0.into(),
        &program_id,
        mint_size_config,
        compressed_account_address,
        1,
    )?;
    sol_log_compute_units();
    // 3. Execute CPI to light-system-program
    // Extract tree accounts for the generalized CPI call
    let tree_accounts = [*accounts[9].key, *accounts[10].key]; // address_merkle_tree, output_queue

    execute_cpi_invoke(
        accounts,
        cpi_bytes,
        &tree_accounts,
        false, // no sol_pool_pda for create_compressed_mint
        None,  // no cpi_context_account for create_compressed_mint
    )
}

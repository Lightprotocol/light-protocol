use anchor_lang::{prelude::msg, solana_program::program_error::ProgramError};
use light_compressed_account::{
    address::derive_address,
    compressed_account::{CompressedAccountConfig, CompressedAccountDataConfig},
    instruction_data::{
        compressed_proof::CompressedProofConfig,
        cpi_context::CompressedCpiContextConfig,
        data::{NewAddressParamsPackedConfig, OutputCompressedAccountWithPackedContextConfig},
        invoke_cpi::{InstructionDataInvokeCpi, InstructionDataInvokeCpiConfig},
        with_readonly::{
            InstructionDataInvokeCpiWithReadOnly, InstructionDataInvokeCpiWithReadOnlyConfig,
        },
    },
    Pubkey,
};
use light_sdk_pinocchio::NewAddressParamsAssignedPackedConfig;
use light_zero_copy::borsh::Deserialize;
use pinocchio::account_info::AccountInfo;
use spl_token::solana_program::log::sol_log_compute_units;

use crate::{
    extensions::{
        metadata_pointer::InitializeMetadataPointerInstructionData,
        processor::process_create_extensions, ExtensionType,
    },
    mint::{
        accounts::CreateCompressedMintAccounts,
        instructions::CreateCompressedMintInstructionData,
        output::create_output_compressed_mint_account,
        state::{CompressedMint, CompressedMintConfig},
    },
    shared::cpi::execute_cpi_invoke,
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
        CreateCompressedMintAccounts::validate_and_parse(accounts, &program_id.into())?;
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
    use light_zero_copy::ZeroCopyNew;

    let mint_size_config: <CompressedMint as ZeroCopyNew>::ZeroCopyConfig = CompressedMintConfig {
        mint_authority: (true, ()),
        freeze_authority: (parsed_instruction_data.freeze_authority.is_some(), ()),
    };
    let compressed_mint_len = CompressedMint::byte_len(&mint_size_config) as u32;
    let mut output_compressed_accounts = vec![OutputCompressedAccountWithPackedContextConfig {
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
    let mut new_address_params = vec![NewAddressParamsAssignedPackedConfig {}];
    if parsed_instruction_data.extensions.is_some() {
        for extension in parsed_instruction_data.extensions.as_ref().unwrap().iter() {
            match ExtensionType::try_from(extension.extension_type).unwrap() {
                ExtensionType::MetadataPointer => {
                    let (extension, token_metadata) =
                        InitializeMetadataPointerInstructionData::zero_copy_at(extension.data)
                            .map_err(|_| ProgramError::InvalidInstructionData)?;
                    let mut data_len = 0;
                    if extension.authority.is_some() {
                        data_len += 33;
                    } else {
                        data_len += 1;
                    };
                    if extension.metadata_address_params.is_some() {
                        data_len += 33;
                    } else {
                        data_len += 1;
                    };
                    // increased mint account data len
                    output_compressed_accounts[0].compressed_account.data.1.data += data_len;
                    // set token metadata account data len
                    if !token_metadata.is_empty() {
                        new_address_params.push(NewAddressParamsAssignedPackedConfig {});
                        output_compressed_accounts.push(
                            OutputCompressedAccountWithPackedContextConfig {
                                compressed_account: CompressedAccountConfig {
                                    address: (true, ()),
                                    data: (
                                        true,
                                        CompressedAccountDataConfig {
                                            data: token_metadata.len() as u32,
                                        },
                                    ),
                                },
                            },
                        );
                    }
                }
                _ => return Err(ProgramError::InvalidInstructionData),
            }
        }
    }
    let final_compressed_mint_len = output_compressed_accounts[0].compressed_account.data.1.data;
    let config = InstructionDataInvokeCpiWithReadOnlyConfig {
        cpi_context: CompressedCpiContextConfig {},
        input_compressed_accounts: vec![],
        proof: (true, CompressedProofConfig {}),
        read_only_accounts: vec![],
        read_only_addresses: vec![],
        new_address_params,
        output_compressed_accounts,
    };
    // TODO: InstructionDataInvokeCpi::Output -> InstructionDataInvokeCpi::ZeroCopyMut and InstructionDataInvokeCpi::ZeroCopy
    // TODO: hardcode since len is constant
    let vec_len = InstructionDataInvokeCpiWithReadOnly::byte_len(&config);
    msg!("vec len {}", vec_len);
    // + discriminator len + vector len
    let mut cpi_bytes = vec![0u8; vec_len + 8 + 4];
    cpi_bytes[0..8]
        .copy_from_slice(&light_compressed_account::discriminators::DISCRIMINATOR_INVOKE_CPI);
    cpi_bytes[8..12].copy_from_slice(&(vec_len as u32).to_le_bytes());

    sol_log_compute_units();
    let (mut cpi_instruction_struct, _) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[12..], config)
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
    cpi_instruction_struct.new_address_params[0].assigned_account_index = 0;
    // Note we can skip address derivation since we are assigning it to the account in index 0.
    cpi_instruction_struct.new_address_params[0].assigned_to_account = 1;
    // 2. process token extensions.
    if let Some(extensions) = parsed_instruction_data.extensions.as_ref() {
        process_create_extensions(
            extensions,
            &mut cpi_instruction_struct,
            final_compressed_mint_len as usize,
        )?;
    }
    // 2. Create compressed mint account data
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
    )?;
    sol_log_compute_units();
    // 3. Execute CPI to light-system-program
    // Extract tree accounts for the generalized CPI call
    let tree_accounts = [accounts[10].key(), accounts[11].key()]; // address_merkle_tree, output_queue
    let _accounts = accounts[1..]
        .iter()
        .map(|account| account.key())
        .collect::<Vec<_>>();
    msg!("tree_accounts {:?}", tree_accounts);
    msg!("accounts {:?}", _accounts);
    execute_cpi_invoke(
        &accounts[2..], // Skip first non-CPI account (mint_signer)
        cpi_bytes,
        tree_accounts.as_slice(),
        false, // no sol_pool_pda for create_compressed_mint
        None,  // no cpi_context_account for create_compressed_mint
    )
}

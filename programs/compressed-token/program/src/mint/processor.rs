use account_compression::utils::constants::NOOP_PUBKEY;
use anchor_lang::{
    prelude::{msg, AccountMeta},
    solana_program::{account_info::AccountInfo, program_error::ProgramError},
    Key,
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
use light_sdk::cpi::invoke_light_system_program;
use light_sdk_types::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, LIGHT_SYSTEM_PROGRAM_ID, REGISTERED_PROGRAM_PDA,
};
use light_zero_copy::{borsh::Deserialize, borsh_mut::DeserializeMut, ZeroCopyNew};
use spl_token::solana_program::log::sol_log_compute_units;

use crate::{
    constants::COMPRESSED_MINT_DISCRIMINATOR,
    mint::{
        accounts::CreateCompressedMintAccounts,
        instructions::{CreateCompressedMintInstructionData, ZCreateCompressedMintInstructionData},
        state::{CompressedMint, CompressedMintConfig},
    },
    LIGHT_CPI_SIGNER,
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
    let mint_pda = solana_pubkey::Pubkey::create_program_address(
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
    // 2. Create compressed mint account data
    create_compressed_mint_account(
        &mut cpi_instruction_struct,
        mint_pda,
        parsed_instruction_data,
        validated_accounts.address_merkle_tree.key.into(),
        &program_id,
        mint_size_config,
    )?;
    sol_log_compute_units();
    // // 3. Execute CPI to light-system-program
    execute_cpi_invoke(accounts, cpi_bytes)
}

fn create_compressed_mint_account(
    cpi_struct: &mut <InstructionDataInvokeCpi as DeserializeMut>::Output,
    mint_pda: Pubkey,
    parsed_instruction_data: ZCreateCompressedMintInstructionData,
    address_merkle_tree_key: Pubkey,
    program_id: &Pubkey,
    mint_config: CompressedMintConfig,
) -> Result<(), ProgramError> {
    if let Some(proof) = cpi_struct.proof.as_deref_mut() {
        proof.a = parsed_instruction_data.proof.a;
        proof.b = parsed_instruction_data.proof.b;
        proof.c = parsed_instruction_data.proof.c;
    }
    // 1. Create NewAddressParams
    cpi_struct.new_address_params[0].seed = mint_pda.to_bytes();
    cpi_struct.new_address_params[0].address_merkle_tree_root_index =
        *parsed_instruction_data.address_merkle_tree_root_index;

    // 2. Derive compressed account address
    let compressed_account_address = derive_address(
        &mint_pda.to_bytes(),
        &address_merkle_tree_key.to_bytes(),
        &program_id.to_bytes(),
    );

    // 3. Create output compressed account
    {
        // TODO: create helper to assign output_compressed_account
        cpi_struct.output_compressed_accounts[0]
            .compressed_account
            .owner = *program_id;

        if let Some(address) = cpi_struct.output_compressed_accounts[0]
            .compressed_account
            .address
            .as_deref_mut()
        {
            *address = compressed_account_address;
        } else {
            panic!("Compressed account address is required");
        }
        *cpi_struct.output_compressed_accounts[0].merkle_tree_index = 1;
    }
    // 4. Create CompressedMint account data & compute hash
    {
        // TODO: create helper to assign compressed account data
        let compressed_account_data = cpi_struct.output_compressed_accounts[0]
            .compressed_account
            .data
            .as_mut()
            .ok_or(ProgramError::InvalidAccountData)?;

        compressed_account_data.discriminator = COMPRESSED_MINT_DISCRIMINATOR;
        let (mut compressed_mint, _) =
            CompressedMint::new_zero_copy(compressed_account_data.data, mint_config)
                .map_err(ProgramError::from)?;
        compressed_mint.spl_mint = mint_pda;
        compressed_mint.decimals = parsed_instruction_data.decimals;
        if let Some(z_freeze_authority) = compressed_mint.freeze_authority.as_deref_mut() {
            *z_freeze_authority = *(parsed_instruction_data
                .freeze_authority
                .as_deref()
                .ok_or(ProgramError::InvalidAccountData)?);
        }
        if let Some(z_mint_authority) = compressed_mint.mint_authority.as_deref_mut() {
            *z_mint_authority = parsed_instruction_data.mint_authority;
        }

        *compressed_account_data.data_hash = compressed_mint
            .hash()
            .map_err(|_| ProgramError::InvalidAccountData)?;
    }

    Ok(())
}

fn execute_cpi_invoke<'info>(
    accounts: &'info [AccountInfo<'info>],
    cpi_bytes: Vec<u8>,
) -> Result<(), ProgramError> {
    // Account order must match light-system program's InvokeCpiInstruction expectation:
    // 0: fee_payer, 1: authority, 2: registered_program_pda, 3: noop_program,
    // 4: account_compression_authority, 5: account_compression_program, 6: invoking_program,
    // 7: sol_pool_pda (optional), 8: decompression_recipient (optional), 9: system_program,
    // 10: cpi_context_account (optional), then remaining accounts (merkle trees, etc.)
    let account_metas = vec![
        AccountMeta::new(accounts[0].key(), true), // fee_payer (signer, mutable)
        AccountMeta::new_readonly(LIGHT_CPI_SIGNER.cpi_signer.into(), true), // authority (cpi_authority_pda)
        AccountMeta::new_readonly(REGISTERED_PROGRAM_PDA.into(), false), // registered_program_pda
        AccountMeta::new_readonly(NOOP_PUBKEY.into(), false),            // noop_program
        AccountMeta::new_readonly(ACCOUNT_COMPRESSION_AUTHORITY_PDA.into(), false), // account_compression_authority
        AccountMeta::new_readonly(account_compression::ID, false), // account_compression_program
        AccountMeta::new_readonly(crate::ID, false), // invoking_program (self_program)
        AccountMeta::new_readonly(LIGHT_SYSTEM_PROGRAM_ID.into(), false), // sol_pool_pda (None, using default)
        AccountMeta::new_readonly(LIGHT_SYSTEM_PROGRAM_ID.into(), false), // decompression_recipient (None, using default)
        AccountMeta::new_readonly(Pubkey::default().into(), false),       // system_program
        AccountMeta::new_readonly(LIGHT_SYSTEM_PROGRAM_ID.into(), false), // cpi_context_account (None, using default)
        AccountMeta::new(accounts[9].key(), false), // address_merkle_tree (mutable)
        AccountMeta::new(accounts[10].key(), false), // output_queue (mutable)
    ];
    let instruction = anchor_lang::solana_program::instruction::Instruction {
        program_id: LIGHT_SYSTEM_PROGRAM_ID.into(),
        accounts: account_metas,
        data: cpi_bytes,
    };
    invoke_light_system_program(accounts, instruction, LIGHT_CPI_SIGNER.bump)?;

    Ok(())
}

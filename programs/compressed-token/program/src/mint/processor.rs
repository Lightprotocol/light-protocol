use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::{
    instruction_data::with_readonly::InstructionDataInvokeCpiWithReadOnly, Pubkey,
};
use light_ctoken_types::{
    context::TokenContext,
    instructions::create_compressed_mint::CreateCompressedMintInstructionData,
    COMPRESSED_MINT_SEED,
};
use light_zero_copy::{borsh::Deserialize, ZeroCopyNew};
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;
use spl_token::solana_program::log::sol_log_compute_units;

use crate::{
    mint::{
        accounts::CreateCompressedMintAccounts, mint_output::create_output_compressed_mint_account,
        zero_copy_config::get_zero_copy_configs,
    },
    shared::{cpi::execute_cpi_invoke, cpi_bytes_size::allocate_invoke_with_read_only_cpi_bytes},
};

/// Checks:
/// 1. check mint_signer (compressed mint randomness) is signer
/// 2.
pub fn process_create_compressed_mint(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    sol_log_compute_units();
    let (parsed_instruction_data, _) =
        CreateCompressedMintInstructionData::zero_copy_at(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;
    sol_log_compute_units();
    // TODO: refactor cpi context struct we don't need the index in the struct.
    let with_cpi_context = parsed_instruction_data.cpi_context.is_some();
    let write_to_cpi_context = parsed_instruction_data
        .cpi_context
        .as_ref()
        .map(|x| x.first_set_context || x.set_context)
        .unwrap_or_default();
    msg!("Parsed instruction data: {:?}", parsed_instruction_data);
    msg!("write_to_cpi_context: {}", write_to_cpi_context);
    // Validate and parse accounts
    let validated_accounts = CreateCompressedMintAccounts::validate_and_parse(
        accounts,
        with_cpi_context,
        write_to_cpi_context,
    )?;
    sol_log_compute_units();

    // 1. Create spl mint PDA using provided bump
    // - The compressed address is derived from the spl_mint_pda.
    // - The spl mint pda is used as mint in compressed token accounts.
    let spl_mint_pda: Pubkey = solana_pubkey::Pubkey::create_program_address(
        &[
            COMPRESSED_MINT_SEED,
            validated_accounts.mint_signer.key().as_slice(),
            &[parsed_instruction_data.mint_bump],
        ],
        &crate::ID,
    )?
    .into();
    // TODO: hash the address instead of

    let (mint_size_config, config) = get_zero_copy_configs(&parsed_instruction_data)?;

    // + discriminator len + vector len
    let mut cpi_bytes = allocate_invoke_with_read_only_cpi_bytes(&config);

    sol_log_compute_units();
    let (mut cpi_instruction_struct, _) =
        InstructionDataInvokeCpiWithReadOnly::new_zero_copy(&mut cpi_bytes[8..], config)
            .map_err(ProgramError::from)?;
    cpi_instruction_struct.initialize(
        crate::LIGHT_CPI_SIGNER.bump,
        &crate::LIGHT_CPI_SIGNER.program_id.into(),
        Some(parsed_instruction_data.proof),
        parsed_instruction_data.cpi_context,
    )?;

    sol_log_compute_units();
    // 2. Create NewAddressParams
    let address_merkle_tree_account_index = 0;
    let assigned_account_index = 0;
    cpi_instruction_struct.new_address_params[0].set(
        spl_mint_pda.to_bytes(),
        *parsed_instruction_data.address_merkle_tree_root_index,
        None,
        address_merkle_tree_account_index,
    );
    // 3. Create compressed mint account data
    // TODO: add input struct, try to use CompressedMintInput
    // TODO: bench performance input struct vs direct inputs.
    let mut token_context = TokenContext::new();
    create_output_compressed_mint_account(
        &mut cpi_instruction_struct.output_compressed_accounts[0],
        spl_mint_pda,
        parsed_instruction_data.decimals,
        parsed_instruction_data.freeze_authority.map(|fa| *fa),
        Some(parsed_instruction_data.mint_authority),
        0.into(),
        mint_size_config,
        *parsed_instruction_data.mint_address,
        1,
        parsed_instruction_data.version,
        false, // Set is_decompressed = false for new mint creation
        parsed_instruction_data.extensions.as_deref(),
        &mut token_context,
    )?;
    sol_log_compute_units();
    if let Some(trees) = validated_accounts.trees.as_ref() {
        // 4. Execute CPI to light-system-program
        execute_cpi_invoke(
            &accounts[CreateCompressedMintAccounts::CPI_ACCOUNTS_OFFSET..],
            cpi_bytes,
            trees.pubkeys().as_slice(),
            false, // no sol_pool_pda for create_compressed_mint
            None,
            validated_accounts
                .system
                .as_ref()
                .unwrap()
                .cpi_context
                .map(|x| *x.key()),
            false, // write to cpi context account
        )
    } else {
        execute_cpi_invoke(
            &accounts[CreateCompressedMintAccounts::CPI_ACCOUNTS_OFFSET..],
            cpi_bytes,
            &[],
            false, // no sol_pool_pda for create_compressed_mint
            None,
            validated_accounts
                .cpi_context_light_system_accounts
                .as_ref()
                .map(|x| *x.cpi_context.key()),
            true,
        )
    }
}

use anchor_lang::solana_program::{
    program_error::ProgramError, rent::Rent, system_instruction, sysvar::Sysvar,
};

use light_ctoken_types::COMPRESSED_MINT_SEED;

use crate::{constants::POOL_SEED, LIGHT_CPI_SIGNER};
/*
// TODO: add test which asserts spl mint and compressed mint equivalence.
// TODO: check and handle extensions
pub fn process_create_spl_mint(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    sol_log_compute_units();

    // Parse instruction data using zero-copy
    let (parsed_instruction_data, _) = CreateSplMintInstructionData::zero_copy_at(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    sol_log_compute_units();
    let with_cpi_context = parsed_instruction_data.cpi_context();
    // Validate and parse accounts
    let validated_accounts = CreateSplMintAccounts::validate_and_parse(accounts, with_cpi_context)?;

    // Check mint authority if it exists.
    if let Some(ix_data_mint_authority) = parsed_instruction_data.mint.mint.mint_authority {
        if *validated_accounts.authority.key() != ix_data_mint_authority.to_bytes() {
            return Err(ProgramError::InvalidAccountData);
        }
    }
    // Verify mint PDA matches the spl_mint field in compressed mint inputs
    // TODO: set it instead of passing it, to eliminate duplicate ix data.
    let expected_mint: [u8; 32] = parsed_instruction_data.mint.mint.spl_mint.to_bytes();
    if validated_accounts.mint.key() != &expected_mint {
        return Err(ProgramError::InvalidAccountData);
    }

    // Create the mint account manually (PDA derived from our program, owned by token program)
    create_mint_account(
        &validated_accounts,
        &crate::LIGHT_CPI_SIGNER.program_id,
        parsed_instruction_data.mint_bump,
    )?;

    // Initialize the mint account using Token-2022's initialize_mint2 instruction
    initialize_mint_account(&validated_accounts, &parsed_instruction_data)?;

    // Create the token pool account manually (PDA derived from our program, owned by token program)
    create_token_pool_account_manual(&validated_accounts, &crate::LIGHT_CPI_SIGNER.program_id)?;

    // Initialize the token pool account
    initialize_token_pool_account(&validated_accounts)?;

    // Mint the existing supply to the token pool if there's any supply
    if parsed_instruction_data.mint.mint.supply > 0 {
        mint_to_token_pool(
            validated_accounts.mint,
            validated_accounts.token_pool_pda,
            validated_accounts.token_program,
            validated_accounts.cpi_authority_pda,
            parsed_instruction_data.mint.mint.supply.into(),
        )?;
    }
    if parsed_instruction_data.mint_authority_is_none() {
        // TODO: remove mint authority from spl mint.
    }

    // Update the compressed mint to mark it as is_decompressed = true
    update_compressed_mint_to_decompressed(
        accounts,
        &validated_accounts,
        &parsed_instruction_data,
        with_cpi_context,
    )?;

    sol_log_compute_units();
    Ok(())
}

const IN_TREE: u8 = 0;
const IN_OUTPUT_QUEUE: u8 = 1;
const OUT_OUTPUT_QUEUE: u8 = 2;

const IN_TREE: u8 = 0;
const IN_OUTPUT_QUEUE: u8 = 1;
const OUT_OUTPUT_QUEUE: u8 = 2;
}
*/

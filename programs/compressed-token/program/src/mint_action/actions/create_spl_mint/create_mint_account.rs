use anchor_lang::solana_program::program_error::ProgramError;
use light_ctoken_types::COMPRESSED_MINT_SEED;

use crate::LIGHT_CPI_SIGNER;

/// Creates the mint account manually as a PDA derived from our program but owned by the token program
pub fn create_mint_account(
    executing_accounts: &crate::mint_action::accounts::ExecutingAccounts<'_>,
    program_id: &pinocchio::pubkey::Pubkey,
    mint_bump: u8,
    mint_signer: &pinocchio::account_info::AccountInfo,
) -> Result<(), ProgramError> {
    let mint_account_size = light_ctoken_types::MINT_ACCOUNT_SIZE as usize;
    let mint_account = executing_accounts
        .mint
        .ok_or(ProgramError::InvalidAccountData)?;
    let token_program = executing_accounts
        .token_program
        .ok_or(ProgramError::InvalidAccountData)?;

    // Verify the provided mint account matches the expected PDA
    let seeds = &[COMPRESSED_MINT_SEED, mint_signer.key().as_ref()];
    crate::shared::verify_pda(mint_account.key(), seeds, mint_bump, program_id)?;

    // Create account using shared function
    let config = crate::shared::CreatePdaAccountConfig {
        seeds,
        bump: mint_bump,
        account_size: mint_account_size,
        owner_program_id: token_program.key(), // Owned by token program
        derivation_program_id: program_id,
    };

    crate::shared::create_pda_account(
        executing_accounts.system.fee_payer,
        mint_account,
        executing_accounts.system.system_program,
        config,
    )
}

/// Initializes the mint account using Token-2022's initialize_mint2 instruction
pub fn initialize_mint_account_for_action(
    executing_accounts: &crate::mint_action::accounts::ExecutingAccounts<'_>,
    mint_data: &light_ctoken_types::instructions::mint_action::ZCompressedMintInstructionData<'_>,
) -> Result<(), ProgramError> {
    let mint_account = executing_accounts
        .mint
        .ok_or(ProgramError::InvalidAccountData)?;
    let token_program = executing_accounts
        .token_program
        .ok_or(ProgramError::InvalidAccountData)?;

    let spl_ix = spl_token_2022::instruction::initialize_mint2(
        &solana_pubkey::Pubkey::new_from_array(*token_program.key()),
        &solana_pubkey::Pubkey::new_from_array(*mint_account.key()),
        // cpi_signer is spl mint authority for compressed mints.
        // So that the program can ensure cmint and spl mint supply is consistent.
        &solana_pubkey::Pubkey::new_from_array(LIGHT_CPI_SIGNER.cpi_signer),
        // Control that the token pool cannot be frozen.
        Some(&solana_pubkey::Pubkey::new_from_array(
            LIGHT_CPI_SIGNER.cpi_signer,
        )),
        mint_data.decimals,
    )?;

    let initialize_mint_ix = pinocchio::instruction::Instruction {
        program_id: token_program.key(),
        accounts: &[pinocchio::instruction::AccountMeta::new(
            mint_account.key(),
            true,
            false,
        )],
        data: &spl_ix.data,
    };

    match pinocchio::program::invoke(&initialize_mint_ix, &[mint_account]) {
        Ok(()) => {}
        Err(e) => {
            return Err(ProgramError::Custom(u64::from(e) as u32));
        }
    }

    Ok(())
}

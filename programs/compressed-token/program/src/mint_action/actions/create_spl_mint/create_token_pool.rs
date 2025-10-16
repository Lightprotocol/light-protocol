use anchor_lang::solana_program::program_error::ProgramError;
use light_program_profiler::profile;
use pinocchio::{
    instruction::{AccountMeta, Seed},
    pubkey::Pubkey,
};

use crate::{
    constants::POOL_SEED, mint_action::accounts::ExecutingAccounts, shared::create_pda_account,
};

/// Creates the token pool account manually as a PDA derived from our program but owned by the token program
#[profile]
pub fn create_token_pool_account_manual(
    executing_accounts: &ExecutingAccounts<'_>,
    _program_id: &Pubkey,
    token_pool_bump: u8,
) -> Result<(), ProgramError> {
    let token_account_size = light_ctoken_types::BASE_TOKEN_ACCOUNT_SIZE as usize;

    // Get required accounts
    let mint_account = executing_accounts
        .mint
        .ok_or(ProgramError::InvalidAccountData)?;
    let token_pool_pda = executing_accounts
        .token_pool_pda
        .ok_or(ProgramError::InvalidAccountData)?;
    let _token_program = executing_accounts
        .token_program
        .ok_or(ProgramError::InvalidAccountData)?;

    // Find the bump for verification
    let mint_key = mint_account.key();
    // let program_id_pubkey = solana_pubkey::Pubkey::new_from_array(*program_id);
    // let (expected_token_pool, bump) = solana_pubkey::Pubkey::find_program_address(
    //     &[POOL_SEED, mint_key.as_ref()],
    //     &program_id_pubkey,
    // );

    // // Verify the provided token pool account matches the expected PDA
    // if token_pool_pda.key() != &expected_token_pool.to_bytes() {
    //     return Err(ProgramError::InvalidAccountData);
    // }

    // Create account using shared function
    let bump_seed = [token_pool_bump];
    let seeds = [
        Seed::from(POOL_SEED),
        Seed::from(mint_key.as_ref()),
        Seed::from(bump_seed.as_ref()),
    ];

    let seeds_inputs = [seeds.as_slice()];

    create_pda_account(
        executing_accounts.system.fee_payer,
        token_pool_pda,
        token_account_size,
        seeds_inputs,
        None,
    )
}

/// Initializes the token pool account (assumes account already exists)
pub fn initialize_token_pool_account_for_action(
    executing_accounts: &ExecutingAccounts<'_>,
) -> Result<(), ProgramError> {
    let mint_account = executing_accounts
        .mint
        .ok_or(ProgramError::InvalidAccountData)?;
    let token_pool_pda = executing_accounts
        .token_pool_pda
        .ok_or(ProgramError::InvalidAccountData)?;
    let token_program = executing_accounts
        .token_program
        .ok_or(ProgramError::InvalidAccountData)?;

    let initialize_account_ix = pinocchio::instruction::Instruction {
        program_id: token_program.key(),
        accounts: &[
            AccountMeta::new(token_pool_pda.key(), true, false),
            AccountMeta::readonly(mint_account.key()),
        ],
        data: &spl_token_2022::instruction::initialize_account3(
            &solana_pubkey::Pubkey::new_from_array(*token_program.key()),
            &solana_pubkey::Pubkey::new_from_array(*token_pool_pda.key()),
            &solana_pubkey::Pubkey::new_from_array(*mint_account.key()),
            &solana_pubkey::Pubkey::new_from_array(
                *executing_accounts.system.cpi_authority_pda.key(),
            ),
        )?
        .data,
    };

    match pinocchio::program::invoke(&initialize_account_ix, &[token_pool_pda, mint_account]) {
        Ok(()) => Ok(()),
        Err(e) => Err(ProgramError::Custom(u64::from(e) as u32)),
    }
}

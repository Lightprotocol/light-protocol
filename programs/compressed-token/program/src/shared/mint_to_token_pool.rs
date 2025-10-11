use anchor_lang::solana_program::program_error::ProgramError;
use light_program_profiler::profile;
use light_sdk_types::CPI_AUTHORITY_PDA_SEED;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction, Seed, Signer},
    program::invoke_signed,
};

use crate::LIGHT_CPI_SIGNER;

/// Mint tokens to the token pool using SPL token mint_to instruction.
/// This function is shared between create_spl_mint and mint_to_compressed processors
/// to ensure consistent token pool management.
#[profile]
pub fn mint_to_token_pool(
    mint_account: &AccountInfo,
    token_pool_account: &AccountInfo,
    token_program: &AccountInfo,
    cpi_authority_pda: &AccountInfo,
    amount: u64,
) -> Result<(), ProgramError> {
    // Create SPL mint_to instruction
    let spl_mint_to_ix = spl_token_2022::instruction::mint_to(
        &solana_pubkey::Pubkey::new_from_array(*token_program.key()),
        &solana_pubkey::Pubkey::new_from_array(*mint_account.key()),
        &solana_pubkey::Pubkey::new_from_array(*token_pool_account.key()),
        &solana_pubkey::Pubkey::new_from_array(LIGHT_CPI_SIGNER.cpi_signer),
        &[],
        amount,
    )?;

    // Create instruction for CPI call
    let mint_to_ix = Instruction {
        program_id: token_program.key(),
        accounts: &[
            AccountMeta::new(mint_account.key(), true, false), // mint (writable)
            AccountMeta::new(token_pool_account.key(), true, false), // token_pool (writable)
            AccountMeta::new(&LIGHT_CPI_SIGNER.cpi_signer, false, true), // authority (signer)
        ],
        data: &spl_mint_to_ix.data,
    };

    // Create signer seeds for CPI
    let bump_seed = [LIGHT_CPI_SIGNER.bump];
    let seed_array = [
        Seed::from(CPI_AUTHORITY_PDA_SEED),
        Seed::from(bump_seed.as_slice()),
    ];
    let signer = Signer::from(&seed_array);

    // Execute the mint_to CPI call
    invoke_signed(
        &mint_to_ix,
        &[mint_account, token_pool_account, cpi_authority_pda],
        &[signer],
    )
    .map_err(|e| ProgramError::Custom(u64::from(e) as u32 + 6000))
}

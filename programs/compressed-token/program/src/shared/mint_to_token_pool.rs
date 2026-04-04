use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::constants::CPI_AUTHORITY_PDA_SEED;
use light_program_profiler::profile;
use pinocchio::{
    address::Address,
    cpi::{invoke_signed_with_slice, Seed, Signer},
    instruction::{InstructionAccount, InstructionView},
    AccountView as AccountInfo,
};

use crate::{shared::convert_program_error, LIGHT_CPI_SIGNER};

/// Mint tokens to the token pool using SPL token mint_to instruction.
/// This function is used by mint_to_compressed processors
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
        &solana_pubkey::Pubkey::new_from_array(token_program.address().to_bytes()),
        &solana_pubkey::Pubkey::new_from_array(mint_account.address().to_bytes()),
        &solana_pubkey::Pubkey::new_from_array(token_pool_account.address().to_bytes()),
        &solana_pubkey::Pubkey::new_from_array(LIGHT_CPI_SIGNER.cpi_signer),
        &[],
        amount,
    )?;

    // Create instruction for CPI call
    let cpi_signer_addr = Address::from(LIGHT_CPI_SIGNER.cpi_signer);
    let mint_to_ix = InstructionView {
        program_id: token_program.address(),
        accounts: &[
            InstructionAccount::new(mint_account.address(), true, false), // mint (writable)
            InstructionAccount::new(token_pool_account.address(), true, false), // token_pool (writable)
            InstructionAccount::new(&cpi_signer_addr, false, true), // authority (signer)
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
    invoke_signed_with_slice(
        &mint_to_ix,
        &[mint_account, token_pool_account, cpi_authority_pda],
        &[signer],
    )
    .map_err(convert_program_error)
}

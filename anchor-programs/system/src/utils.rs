use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::solana_program::pubkey::Pubkey;

use crate::constants::SOL_POOL_PDA_SEED;

pub fn get_registered_program_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[program_id.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0
}

pub fn get_cpi_authority_pda(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], program_id).0
}

pub fn get_sol_pool_pda() -> Pubkey {
    Pubkey::find_program_address(&[SOL_POOL_PDA_SEED], &crate::ID).0
}

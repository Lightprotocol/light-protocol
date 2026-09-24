use light_compressed_account::constants::ACCOUNT_COMPRESSION_PROGRAM_ID;
use light_program_profiler::profile;
use pinocchio::{
    account_info::AccountInfo,
    pubkey::{find_program_address, Pubkey},
};

use crate::{
    accounts::remaining_account_checks::AcpAccount, constants::CPI_AUTHORITY_PDA_SEED,
    errors::SystemProgramError, processor::sol_compression::SOL_POOL_PDA_SEED,
};
#[profile]
pub fn get_registered_program_pda(program_id: &Pubkey) -> Pubkey {
    find_program_address(&[program_id.as_ref()], &ACCOUNT_COMPRESSION_PROGRAM_ID).0
}

#[profile]
pub fn get_cpi_authority_pda(program_id: &Pubkey) -> Pubkey {
    find_program_address(&[CPI_AUTHORITY_PDA_SEED], program_id).0
}

#[profile]
pub fn get_sol_pool_pda() -> Pubkey {
    find_program_address(&[SOL_POOL_PDA_SEED], &crate::ID).0
}

#[profile]
pub fn get_queue_and_tree_accounts<'b, 'info>(
    accounts: &'b [AcpAccount<'info>],
    queue_index: usize,
    tree_index: usize,
) -> std::result::Result<(&'b AcpAccount<'info>, &'b AcpAccount<'info>), SystemProgramError> {
    Ok((&accounts[queue_index], &accounts[tree_index]))
}

pub fn transfer_lamports_invoke(
    from: &AccountInfo,
    to: &AccountInfo,
    lamports: u64,
) -> crate::Result<()> {
    let instruction = pinocchio_system::instructions::Transfer { from, to, lamports };
    instruction.invoke()
}

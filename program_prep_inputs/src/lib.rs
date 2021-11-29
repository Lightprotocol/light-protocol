pub mod pi_254_instructions;
pub mod pi_254_parsers;
pub mod pi_254_processor;
pub mod pi_254_ranges;
pub mod pi_254_state;
pub mod pi_254_test;
pub mod pre_processor;

pub mod parse_verifyingkey_254;
pub mod state_merkle_tree_roots;
pub mod verifyingkey_254_hc;
use crate::pre_processor::_pre_process_instruction;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

// _pre_process: Logic happens here. Checks ix_id and routes to the right processors files.
// TODO: Decide whether to put the pre_process logic inside lib.rs file or not.
// Especially if we split logic into different programs we don't need a preprocessor.
// _pi_254_process_instruction: Processor for prepared inputs w/ bn254 curve implementation. Calls ix based on ix_id.
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    msg!("IDATa");

    let account = &mut accounts.iter();
    let account1 = next_account_info(account)?; // this one isn't needed
                                                // the remaining storage accounts are being pulled inside inside pre_process/process
    msg!("IDATA: {:?}", _instruction_data[0..20].to_vec());
    // println!("I data: {:?}", _instruction_data);
    _pre_process_instruction(_instruction_data, accounts);

    Ok(())
}
